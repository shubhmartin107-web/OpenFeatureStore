use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ofs_core::entity_key::serialize_entity_key_v3;
use ofs_core::errors::{OfsError, OfsResult};
use ofs_core::traits::{
    FeatureVector, FieldStatus, OnlineReadResponse, OnlineResponseMetadata, OnlineStore,
};
use ofs_core::types::{EntityKey, FeatureViewWithProjection, OnlineWriteRecord};
use redis::aio::ConnectionManager;

/// Redis-based online store.
///
/// Key format: `ofs:{project}:{fv_name}:{hex_entity_key}`
/// Stored as a Redis hash with field → value mappings.
pub struct RedisOnlineStore {
    conn: ConnectionManager,
}

impl RedisOnlineStore {
    /// Create a new Redis online store.
    pub async fn new(connection_string: &str) -> OfsResult<Self> {
        let client =
            redis::Client::open(connection_string).map_err(|e| OfsError::Redis(e.to_string()))?;
        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| OfsError::Redis(e.to_string()))?;
        Ok(Self { conn })
    }

    fn redis_key(project: &str, fv_name: &str, entity_key: &EntityKey) -> String {
        let serialized = hex::encode(serialize_entity_key_v3(entity_key));
        format!("ofs:{}:{}:{}", project, fv_name, serialized)
    }
}

#[async_trait]
impl OnlineStore for RedisOnlineStore {
    async fn online_read(
        &self,
        entity_keys: Vec<EntityKey>,
        features: &[FeatureViewWithProjection],
        project: &str,
    ) -> OfsResult<OnlineReadResponse> {
        let mut all_feature_names = Vec::new();
        let mut all_results = Vec::new();

        for fvp in features {
            let fv_name = &fvp.feature_view.name;
            let feature_names: Vec<String> = fvp
                .projection
                .feature_columns
                .iter()
                .map(|f| f.name.clone())
                .collect();

            for ek in &entity_keys {
                let key = Self::redis_key(project, fv_name, ek);

                // HGETALL to get all fields
                let result: Result<Vec<(String, String)>, redis::RedisError> =
                    redis::cmd("HGETALL")
                        .arg(&key)
                        .query_async(&mut self.conn.clone())
                        .await;

                match result {
                    Ok(fields) => {
                        let mut map: std::collections::HashMap<String, String> =
                            fields.into_iter().collect();
                        let ts_str = map.remove("_event_ts");
                        let ts = ts_str
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc));

                        let mut vals = Vec::new();
                        let mut statuses = Vec::new();
                        let mut event_timestamps = Vec::new();

                        for fname in &feature_names {
                            let col_name = format!("{}__{}", fv_name, fname);
                            all_feature_names.push(col_name);

                            match map.remove(fname) {
                                Some(val) => {
                                    vals.push(val.into_bytes());
                                    statuses.push(FieldStatus::Present);
                                }
                                None => {
                                    vals.push(Vec::new());
                                    statuses.push(FieldStatus::NullValue);
                                }
                            }
                            event_timestamps.push(ts);
                        }

                        all_results.push(FeatureVector {
                            values: vals,
                            statuses,
                            event_timestamps,
                        });
                    }
                    Err(_) => {
                        // Key not found or error
                        let mut statuses = Vec::new();
                        let mut event_timestamps = Vec::new();
                        for fname in &feature_names {
                            let col_name = format!("{}__{}", fv_name, fname);
                            all_feature_names.push(col_name);
                            statuses.push(FieldStatus::NotFound);
                            event_timestamps.push(None);
                        }
                        all_results.push(FeatureVector {
                            values: vec![Vec::new(); feature_names.len()],
                            statuses,
                            event_timestamps,
                        });
                    }
                }
            }
        }

        Ok(OnlineReadResponse {
            metadata: OnlineResponseMetadata {
                feature_names: all_feature_names,
            },
            results: all_results,
        })
    }

    async fn online_write_batch(
        &self,
        data: Vec<OnlineWriteRecord>,
        project: &str,
    ) -> OfsResult<()> {
        let mut conn = self.conn.clone();

        for record in &data {
            let key = Self::redis_key(project, &record.feature_view_name, &record.entity_key);
            let ts_str = record
                .timestamp
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string();

            let mut pipe = redis::pipe();
            pipe.hset(&key, "_event_ts", &ts_str);

            for (field_name, val) in &record.values {
                pipe.hset(&key, field_name, val);
            }

            pipe.query_async::<()>(&mut conn)
                .await
                .map_err(|e| OfsError::Redis(e.to_string()))?;
        }

        Ok(())
    }

    async fn update(
        &self,
        _tables_to_keep: Vec<String>,
        tables_to_delete: Vec<String>,
    ) -> OfsResult<()> {
        let mut conn = self.conn.clone();

        for pattern in &tables_to_delete {
            // Find and delete all keys matching the pattern
            let mut cursor = 0usize;
            loop {
                let result: (usize, Vec<String>) = redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| OfsError::Redis(e.to_string()))?;

                cursor = result.0;
                let keys = result.1;

                if !keys.is_empty() {
                    redis::cmd("DEL")
                        .arg(keys)
                        .query_async::<()>(&mut conn)
                        .await
                        .map_err(|e| OfsError::Redis(e.to_string()))?;
                }

                if cursor == 0 {
                    break;
                }
            }
        }

        Ok(())
    }

    async fn teardown(&self) -> OfsResult<()> {
        let mut conn = self.conn.clone();
        let mut cursor = 0usize;

        loop {
            let result: (usize, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg("ofs:*")
                .arg("COUNT")
                .arg(200)
                .query_async(&mut conn)
                .await
                .map_err(|e| OfsError::Redis(e.to_string()))?;

            cursor = result.0;
            let keys = result.1;

            if !keys.is_empty() {
                redis::cmd("DEL")
                    .arg(keys)
                    .query_async::<()>(&mut conn)
                    .await
                    .map_err(|e| OfsError::Redis(e.to_string()))?;
            }

            if cursor == 0 {
                break;
            }
        }

        Ok(())
    }

    async fn purge_expired(
        &self,
        feature_view_name: &str,
        project: &str,
        cutoff: DateTime<Utc>,
    ) -> OfsResult<u64> {
        let mut conn = self.conn.clone();
        let pattern = format!("ofs:{}:{}:*", project, feature_view_name);
        let mut cursor = 0usize;
        let mut deleted = 0u64;

        loop {
            let result: (usize, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| OfsError::Redis(e.to_string()))?;

            cursor = result.0;
            let keys = result.1;

            for key in &keys {
                let ts_str: Option<String> = redis::cmd("HGET")
                    .arg(key)
                    .arg("_event_ts")
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| OfsError::Redis(e.to_string()))?;

                if let Some(ts_str) = ts_str
                    && let Ok(ts) = DateTime::parse_from_rfc3339(&ts_str)
                    && ts.with_timezone(&Utc) < cutoff
                {
                    redis::cmd("DEL")
                        .arg(key)
                        .query_async::<()>(&mut conn)
                        .await
                        .map_err(|e| OfsError::Redis(e.to_string()))?;
                    deleted += 1;
                }
            }

            if cursor == 0 {
                break;
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ofs_core::types::{EntityKey, Feature, FeatureView, FeatureViewProjection};
    use ofs_core::value_type::ValueType;
    use std::collections::HashMap;

    fn make_fvp(name: &str, features: Vec<&str>) -> FeatureViewWithProjection {
        let fv_features: Vec<Feature> = features
            .iter()
            .map(|f| Feature::new(f, ValueType::String))
            .collect();
        let fv = FeatureView::new(name);
        FeatureViewWithProjection {
            feature_view: fv,
            projection: FeatureViewProjection {
                feature_view_name: name.to_string(),
                feature_view_name_alias: None,
                feature_columns: fv_features.clone(),
                join_key_map: HashMap::new(),
                timestamp_field: None,
                date_partition_column: None,
                created_timestamp_column: None,
                batch_source: None,
                stream_source: None,
                view_type: "FeatureView".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn test_redis_key_format() {
        let ek = EntityKey::new(vec!["driver_id".to_string()]);
        let key = RedisOnlineStore::redis_key("default", "driver_stats", &ek);
        assert!(key.starts_with("ofs:default:driver_stats:"));
    }

    #[tokio::test]
    #[ignore = "Requires running Redis server"]
    async fn test_write_then_read() {
        let store = RedisOnlineStore::new("redis://127.0.0.1:6379")
            .await
            .unwrap();
        let now = Utc::now();

        let ek = EntityKey::new(vec!["driver_id".to_string()]);
        let mut values = HashMap::new();
        values.insert("conv_rate".to_string(), b"0.85".to_vec());

        let record = OnlineWriteRecord {
            entity_key: ek.clone(),
            values,
            timestamp: now,
            feature_view_name: "driver_stats".to_string(),
        };

        store
            .online_write_batch(vec![record], "default")
            .await
            .unwrap();

        let fvp = make_fvp("driver_stats", vec!["conv_rate"]);
        let response = store
            .online_read(vec![ek], &[fvp], "default")
            .await
            .unwrap();

        assert_eq!(response.results[0].values[0], b"0.85");

        store.teardown().await.unwrap();
    }
}
