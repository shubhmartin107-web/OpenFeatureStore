use async_trait::async_trait;
use ofs_core::errors::{OfsError, OfsResult};
use redis::aio::ConnectionManager;

use crate::traits::{CacheKey, CachedValue, FeatureCache};

/// L2 distributed cache backed by Redis.
///
/// Supports single-node Redis and Redis-compatible services.
/// Uses `ConnectionManager` for automatic reconnection.
pub struct L2Cache {
    conn: Option<ConnectionManager>,
    connection_string: String,
    key_prefix: String,
    default_ttl_secs: u64,
}

impl L2Cache {
    pub async fn new(
        connection_string: &str,
        key_prefix: &str,
        default_ttl_secs: u64,
    ) -> OfsResult<Self> {
        let client = redis::Client::open(connection_string)
            .map_err(|e| OfsError::Redis(format!("Failed to create Redis client: {}", e)))?;
        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| OfsError::Redis(format!("Failed to connect to Redis: {}", e)))?;
        Ok(Self {
            conn: Some(conn),
            connection_string: connection_string.to_string(),
            key_prefix: key_prefix.to_string(),
            default_ttl_secs,
        })
    }

    /// Create a new L2 cache without connecting (for testing key format, etc.).
    #[cfg(test)]
    pub fn new_mock(connection_string: &str, key_prefix: &str, default_ttl_secs: u64) -> Self {
        Self {
            conn: None,
            connection_string: connection_string.to_string(),
            key_prefix: key_prefix.to_string(),
            default_ttl_secs,
        }
    }

    fn redis_key(&self, key: &CacheKey) -> String {
        format!(
            "{}ofs:cache:{}:{}:{}",
            self.key_prefix, key.project, key.feature_view, key.entity_key
        )
    }

    async fn conn(&self) -> OfsResult<ConnectionManager> {
        match &self.conn {
            Some(conn) => Ok(conn.clone()),
            None => {
                let client = redis::Client::open(self.connection_string.as_str()).map_err(|e| {
                    OfsError::Redis(format!("Failed to create Redis client: {}", e))
                })?;
                ConnectionManager::new(client)
                    .await
                    .map_err(|e| OfsError::Redis(format!("Failed to connect to Redis: {}", e)))
            }
        }
    }
}

#[async_trait]
impl FeatureCache for L2Cache {
    async fn get(&self, key: &CacheKey) -> OfsResult<Option<CachedValue>> {
        let mut conn = self.conn().await?;
        let rk = self.redis_key(key);
        let data: Option<Vec<u8>> = redis::cmd("GET")
            .arg(&rk)
            .query_async(&mut conn)
            .await
            .map_err(|e| OfsError::Redis(format!("Redis GET failed: {}", e)))?;

        match data {
            Some(bytes) => {
                let value: CachedValue = serde_json::from_slice(&bytes).map_err(|e| {
                    OfsError::Serialization(format!("Failed to deserialize cache value: {}", e))
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn set(&self, key: CacheKey, value: CachedValue) -> OfsResult<()> {
        let mut conn = self.conn().await?;
        let rk = self.redis_key(&key);
        let data = serde_json::to_vec(&value).map_err(|e| {
            OfsError::Serialization(format!("Failed to serialize cache value: {}", e))
        })?;

        redis::cmd("SETEX")
            .arg(&rk)
            .arg(self.default_ttl_secs)
            .arg(&data)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| OfsError::Redis(format!("Redis SETEX failed: {}", e)))?;

        Ok(())
    }

    async fn invalidate(&self, key: &CacheKey) -> OfsResult<()> {
        let mut conn = self.conn().await?;
        let rk = self.redis_key(key);
        redis::cmd("DEL")
            .arg(&rk)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| OfsError::Redis(format!("Redis DEL failed: {}", e)))?;

        Ok(())
    }

    async fn clear(&self) -> OfsResult<()> {
        let mut conn = self.conn().await?;
        let pattern = format!("{}ofs:cache:*", self.key_prefix);
        let mut cursor = 0usize;
        loop {
            let result: (usize, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| OfsError::Redis(format!("Redis SCAN failed: {}", e)))?;

            cursor = result.0;
            let keys = result.1;

            if !keys.is_empty() {
                redis::cmd("DEL")
                    .arg(keys)
                    .query_async::<()>(&mut conn)
                    .await
                    .map_err(|e| OfsError::Redis(format!("Redis DEL failed: {}", e)))?;
            }

            if cursor == 0 {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::*;

    fn make_key(ek: &str) -> CacheKey {
        CacheKey {
            project: "default".to_string(),
            feature_view: "driver_stats".to_string(),
            entity_key: ek.to_string(),
        }
    }

    fn make_value() -> CachedValue {
        CachedValue {
            values: vec![b"0.85".to_vec()],
            statuses: vec![1],
            event_timestamps: vec![Some("2024-01-01T00:00:00Z".to_string())],
            cached_at: "2024-06-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn test_redis_key_format() {
        let cache = L2Cache::new_mock("redis://127.0.0.1:6379", "test_", 300);
        let key = make_key("driver-1");
        let rk = cache.redis_key(&key);
        assert_eq!(rk, "test_ofs:cache:default:driver_stats:driver-1");
    }

    #[tokio::test]
    #[ignore = "Requires running Redis server"]
    async fn test_set_and_get() {
        let cache = L2Cache::new("redis://127.0.0.1:6379", "", 300)
            .await
            .unwrap();
        let key = make_key("driver-1");
        cache.set(key.clone(), make_value()).await.unwrap();
        let got = cache.get(&key).await.unwrap();
        assert!(got.is_some());
        cache.invalidate(&key).await.unwrap();
    }
}
