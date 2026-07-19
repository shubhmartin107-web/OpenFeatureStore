use chrono::{DateTime, Utc};
use ofs_core::errors::{OfsError, OfsResult};
use ofs_core::traits::OnlineStore;
use ofs_core::types::{EntityKey, OnlineWriteRecord};
use std::collections::HashMap;
use std::sync::Arc;
use tracing;

use crate::dlq::{DeadLetterQueue, DlqRecord};
use crate::wal::WriteAheadLog;

/// Kafka consumer engine for stream ingestion.
///
/// Reads feature data from Kafka topics and writes to the online store
/// with dedup via WAL and failed-record capture via DLQ.
#[cfg(feature = "kafka")]
pub struct KafkaIngestEngine {
    online_store: Arc<dyn OnlineStore>,
    wal: WriteAheadLog,
    dlq: DeadLetterQueue,
    project: String,
    running: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(feature = "kafka")]
impl KafkaIngestEngine {
    pub fn new(
        online_store: Arc<dyn OnlineStore>,
        wal: WriteAheadLog,
        dlq: DeadLetterQueue,
        project: &str,
    ) -> Self {
        Self {
            online_store,
            wal,
            dlq,
            project: project.to_string(),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub async fn start(&self, brokers: &str, topic: &str, group_id: &str) -> OfsResult<()> {
        use rdkafka::ClientConfig;
        use rdkafka::consumer::{Consumer, StreamConsumer};
        use rdkafka::message::Message;
        use tokio::stream::StreamExt;

        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", brokers)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("session.timeout.ms", "6000")
            .create()
            .map_err(|e| OfsError::Config(format!("Failed to create Kafka consumer: {}", e)))?;

        consumer.subscribe(&[topic]).map_err(|e| {
            OfsError::Config(format!("Failed to subscribe to topic '{}': {}", topic, e))
        })?;

        tracing::info!(
            brokers = brokers,
            topic = topic,
            group_id = group_id,
            "Kafka consumer started"
        );

        let mut stream = consumer.stream();
        while self.running.load(std::sync::atomic::Ordering::SeqCst) {
            match stream.next().await {
                Some(Ok(msg)) => {
                    if let Err(e) = self.process_message(&msg).await {
                        tracing::warn!("Failed to process Kafka message: {}", e);
                    }
                }
                Some(Err(e)) => {
                    tracing::error!("Kafka consumer error: {}", e);
                }
                None => {
                    tracing::warn!("Kafka stream ended unexpectedly");
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn shutdown(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    async fn process_message(&self, msg: &rdkafka::message::BorrowedMessage<'_>) -> OfsResult<()> {
        let topic = msg.topic().to_string();
        let partition = msg.partition();
        let offset = msg.offset();
        let payload = msg.payload().unwrap_or_default().to_vec();
        let key = msg.key().map(|k| String::from_utf8_lossy(k).to_string());

        let record_id = format!(
            "{}:{}:{}:{}",
            topic,
            partition,
            offset,
            key.as_deref().unwrap_or("null")
        );

        if self.wal.is_duplicate(&record_id).await? {
            tracing::debug!("Skipping duplicate record: {}", record_id);
            return Ok(());
        }

        self.wal.mark_pending(&record_id).await?;

        match self.ingest_record(&payload, &key).await {
            Ok(()) => {
                self.wal.mark_processed(&record_id).await?;
                Ok(())
            }
            Err(e) => {
                let dlq_record = DlqRecord {
                    id: record_id.clone(),
                    topic,
                    partition,
                    offset,
                    key,
                    payload,
                    error: e.to_string(),
                    failed_at: Utc::now(),
                };
                if let Err(dlq_err) = self.dlq.push(dlq_record).await {
                    tracing::error!("Failed to push to DLQ: {}", dlq_err);
                }
                self.wal.mark_failed(&record_id, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    async fn ingest_record(&self, payload: &[u8], key: &Option<String>) -> OfsResult<()> {
        let body: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| OfsError::Config(format!("Invalid JSON payload: {}", e)))?;

        let feature_view = body
            .get("feature_view")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OfsError::Config("Missing 'feature_view' in payload".to_string()))?;

        let entity_values: Vec<String> = body
            .get("entity_key")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .or_else(|| key.as_ref().map(|k| vec![k.clone()]))
            .ok_or_else(|| OfsError::Config("Missing 'entity_key' in payload".to_string()))?;

        let features = body
            .get("features")
            .and_then(|v| v.as_object())
            .ok_or_else(|| OfsError::Config("Missing 'features' in payload".to_string()))?;

        let timestamp_str = body
            .get("event_timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or(&Utc::now().to_rfc3339());

        let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_str)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| OfsError::Config(format!("Invalid timestamp: {}", e)))?;

        let entity_key = EntityKey::new(entity_values);

        let mut values = HashMap::new();
        for (name, val) in features {
            let bytes = serde_json::to_vec(val).map_err(|e| {
                OfsError::Config(format!("Failed to serialize feature '{}': {}", name, e))
            })?;
            values.insert(name.clone(), bytes);
        }

        let record = OnlineWriteRecord {
            entity_key,
            values,
            timestamp,
            feature_view_name: feature_view.to_string(),
        };

        self.online_store
            .online_write_batch(vec![record], &self.project)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ofs_core::traits::OnlineReadResponse;

    struct MockOnlineStore;
    #[async_trait::async_trait]
    impl OnlineStore for MockOnlineStore {
        async fn online_read(
            &self,
            _entity_keys: Vec<EntityKey>,
            _features: &[ofs_core::types::FeatureViewWithProjection],
            _project: &str,
        ) -> OfsResult<OnlineReadResponse> {
            Ok(OnlineReadResponse {
                metadata: ofs_core::traits::OnlineResponseMetadata {
                    feature_names: Vec::new(),
                },
                results: Vec::new(),
            })
        }
        async fn online_write_batch(
            &self,
            _data: Vec<OnlineWriteRecord>,
            _project: &str,
        ) -> OfsResult<()> {
            Ok(())
        }
        async fn update(
            &self,
            _tables_to_keep: Vec<String>,
            _tables_to_delete: Vec<String>,
        ) -> OfsResult<()> {
            Ok(())
        }
        async fn purge_expired(
            &self,
            _feature_view_name: &str,
            _project: &str,
            _cutoff: DateTime<Utc>,
        ) -> OfsResult<u64> {
            Ok(0)
        }
        async fn teardown(&self) -> OfsResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_ingest_record_json() {
        #[cfg(feature = "kafka")]
        {
            let online = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;
            let wal = WriteAheadLog::new(None).await.unwrap();
            let dlq = DeadLetterQueue::new(None).await.unwrap();
            let engine = KafkaIngestEngine::new(online, wal, dlq, "default");

            let payload = br#"{
                "feature_view": "driver_stats",
                "entity_key": ["driver-123"],
                "features": {"conv_rate": 0.85},
                "event_timestamp": "2024-01-01T00:00:00Z"
            }"#;

            let result = engine
                .ingest_record(payload, &Some("driver-123".to_string()))
                .await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_ingest_record_missing_fields() {
        #[cfg(feature = "kafka")]
        {
            let online = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;
            let wal = WriteAheadLog::new(None).await.unwrap();
            let dlq = DeadLetterQueue::new(None).await.unwrap();
            let engine = KafkaIngestEngine::new(online, wal, dlq, "default");

            let payload = br#"{"bad_key": "value"}"#;
            let result = engine.ingest_record(payload, &None).await;
            assert!(result.is_err());
        }
    }
}
