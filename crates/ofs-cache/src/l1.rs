use async_trait::async_trait;
use moka::future::Cache;
use ofs_core::errors::OfsResult;
use std::time::Duration;

use crate::traits::{CacheKey, CachedValue, FeatureCache};

/// L1 in-memory cache backed by moka (concurrent, lock-free LRU).
pub struct L1Cache {
    cache: Cache<CacheKey, CachedValue>,
}

impl L1Cache {
    pub fn new(max_size: u64, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_size)
            .time_to_live(ttl)
            .build();
        Self { cache }
    }
}

#[async_trait]
impl FeatureCache for L1Cache {
    async fn get(&self, key: &CacheKey) -> OfsResult<Option<CachedValue>> {
        Ok(self.cache.get(key).await)
    }

    async fn set(&self, key: CacheKey, value: CachedValue) -> OfsResult<()> {
        self.cache.insert(key, value).await;
        Ok(())
    }

    async fn invalidate(&self, key: &CacheKey) -> OfsResult<()> {
        self.cache.invalidate(key).await;
        Ok(())
    }

    async fn clear(&self) -> OfsResult<()> {
        self.cache.invalidate_all();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::*;
    use std::time::Duration;

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
    async fn test_set_and_get() {
        let cache = L1Cache::new(100, Duration::from_secs(300));
        let key = make_key("driver-1");
        let val = make_value();
        cache.set(key.clone(), val.clone()).await.unwrap();
        let got = cache.get(&key).await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().values[0], b"0.85");
    }

    #[tokio::test]
    async fn test_miss() {
        let cache = L1Cache::new(100, Duration::from_secs(300));
        let key = make_key("nonexistent");
        let got = cache.get(&key).await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn test_invalidate() {
        let cache = L1Cache::new(100, Duration::from_secs(300));
        let key = make_key("driver-1");
        cache.set(key.clone(), make_value()).await.unwrap();
        cache.invalidate(&key).await.unwrap();
        let got = cache.get(&key).await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = L1Cache::new(100, Duration::from_secs(300));
        cache.set(make_key("driver-1"), make_value()).await.unwrap();
        cache.set(make_key("driver-2"), make_value()).await.unwrap();
        cache.clear().await.unwrap();
        assert!(cache.get(&make_key("driver-1")).await.unwrap().is_none());
        assert!(cache.get(&make_key("driver-2")).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_eviction_time_to_live() {
        let cache = L1Cache::new(100, Duration::from_millis(50));
        cache.set(make_key("k1"), make_value()).await.unwrap();
        assert!(cache.get(&make_key("k1")).await.unwrap().is_some());
        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(cache.get(&make_key("k1")).await.unwrap().is_none());
    }
}
