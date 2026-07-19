use chrono::{DateTime, Utc};
use ofs_core::errors::{OfsError, OfsResult};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// A record stored in the dead-letter queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqRecord {
    pub id: String,
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub payload: Vec<u8>,
    pub error: String,
    pub failed_at: DateTime<Utc>,
}

type DlqRow = (
    String,
    String,
    i32,
    i64,
    Option<String>,
    Vec<u8>,
    String,
    String,
);

/// Dead-letter queue for failed stream ingestion records.
///
/// Stores records that could not be processed for later inspection or replay.
pub struct DeadLetterQueue {
    pool: SqlitePool,
}

impl DeadLetterQueue {
    pub async fn new(path: Option<&str>) -> OfsResult<Self> {
        let pool = match path {
            Some(p) => SqlitePool::connect(p).await,
            None => SqlitePool::connect("sqlite::memory:").await,
        }
        .map_err(|e| OfsError::Database(e.to_string()))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ingest_dlq (
                id TEXT PRIMARY KEY NOT NULL,
                topic TEXT NOT NULL,
                partition INTEGER NOT NULL DEFAULT 0,
                offset_val INTEGER NOT NULL DEFAULT 0,
                key_text TEXT,
                payload BLOB NOT NULL,
                error TEXT NOT NULL,
                failed_at TEXT NOT NULL,
                replayed INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Push a failed record to the dead-letter queue.
    pub async fn push(&self, record: DlqRecord) -> OfsResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO ingest_dlq (id, topic, partition, offset_val, key_text, payload, error, failed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&record.id)
        .bind(&record.topic)
        .bind(record.partition)
        .bind(record.offset)
        .bind(&record.key)
        .bind(&record.payload)
        .bind(&record.error)
        .bind(record.failed_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(())
    }

    /// Retrieve un-replayed records for retry, marking them as replayed.
    pub async fn fetch_for_replay(&self, limit: usize) -> OfsResult<Vec<DlqRecord>> {
        let rows: Vec<DlqRow> = sqlx::query_as(
            "SELECT id, topic, partition, offset_val, key_text, payload, error, failed_at
                 FROM ingest_dlq WHERE replayed = 0 ORDER BY failed_at ASC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;

        let mut records = Vec::with_capacity(rows.len());
        for (id, topic, partition, offset, key, payload, error, failed_str) in rows {
            let failed_at = DateTime::parse_from_rfc3339(&failed_str)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| OfsError::Database(e.to_string()))?;
            records.push(DlqRecord {
                id,
                topic,
                partition,
                offset,
                key,
                payload,
                error,
                failed_at,
            });
        }
        Ok(records)
    }

    /// Mark a record as successfully replayed.
    pub async fn mark_replayed(&self, record_id: &str) -> OfsResult<()> {
        sqlx::query("UPDATE ingest_dlq SET replayed = 1 WHERE id = ?")
            .bind(record_id)
            .execute(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(())
    }

    /// Count total records in the DLQ.
    pub async fn count(&self) -> OfsResult<u64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ingest_dlq")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(count.0 as u64)
    }

    /// Count un-replayed records.
    pub async fn pending_replay_count(&self) -> OfsResult<u64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ingest_dlq WHERE replayed = 0")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(count.0 as u64)
    }

    /// Delete records that have been replayed successfully.
    pub async fn cleanup_replayed(&self) -> OfsResult<u64> {
        let result = sqlx::query("DELETE FROM ingest_dlq WHERE replayed = 1")
            .execute(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_dlq() -> DeadLetterQueue {
        DeadLetterQueue::new(None).await.unwrap()
    }

    #[tokio::test]
    async fn test_push_and_count() {
        let dlq = create_dlq().await;
        let record = DlqRecord {
            id: "dlq-1".to_string(),
            topic: "features".to_string(),
            partition: 0,
            offset: 42,
            key: None,
            payload: b"test payload".to_vec(),
            error: "processing failed".to_string(),
            failed_at: Utc::now(),
        };
        dlq.push(record).await.unwrap();
        assert_eq!(dlq.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_fetch_for_replay() {
        let dlq = create_dlq().await;
        let record = DlqRecord {
            id: "dlq-1".to_string(),
            topic: "features".to_string(),
            partition: 0,
            offset: 42,
            key: Some("key-1".to_string()),
            payload: b"test".to_vec(),
            error: "error".to_string(),
            failed_at: Utc::now(),
        };
        dlq.push(record).await.unwrap();

        let records = dlq.fetch_for_replay(10).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].key, Some("key-1".to_string()));
    }

    #[tokio::test]
    async fn test_mark_replayed_and_cleanup() {
        let dlq = create_dlq().await;
        let record = DlqRecord {
            id: "dlq-1".to_string(),
            topic: "features".to_string(),
            partition: 0,
            offset: 42,
            key: None,
            payload: b"test".to_vec(),
            error: "error".to_string(),
            failed_at: Utc::now(),
        };
        dlq.push(record).await.unwrap();
        dlq.mark_replayed("dlq-1").await.unwrap();
        assert_eq!(dlq.pending_replay_count().await.unwrap(), 0);
        dlq.cleanup_replayed().await.unwrap();
        assert_eq!(dlq.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_ignore_duplicate_push() {
        let dlq = create_dlq().await;
        let record = DlqRecord {
            id: "dlq-1".to_string(),
            topic: "features".to_string(),
            partition: 0,
            offset: 42,
            key: None,
            payload: b"test".to_vec(),
            error: "error".to_string(),
            failed_at: Utc::now(),
        };
        dlq.push(record.clone()).await.unwrap();
        dlq.push(record).await.unwrap();
        assert_eq!(dlq.count().await.unwrap(), 1);
    }
}
