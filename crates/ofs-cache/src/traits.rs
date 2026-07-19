use async_trait::async_trait;
use ofs_core::errors::OfsResult;
use serde::{Deserialize, Serialize};

/// Key used to identify a cached feature value.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct CacheKey {
    pub project: String,
    pub feature_view: String,
    pub entity_key: String,
}

impl std::fmt::Display for CacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.project, self.feature_view, self.entity_key
        )
    }
}

/// A cached feature vector value with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedValue {
    pub values: Vec<Vec<u8>>,
    pub statuses: Vec<i32>,
    pub event_timestamps: Vec<Option<String>>,
    pub cached_at: String,
}

/// Trait for feature cache implementations.
#[async_trait]
pub trait FeatureCache: Send + Sync {
    async fn get(&self, key: &CacheKey) -> OfsResult<Option<CachedValue>>;
    async fn set(&self, key: CacheKey, value: CachedValue) -> OfsResult<()>;
    async fn invalidate(&self, key: &CacheKey) -> OfsResult<()>;
    async fn clear(&self) -> OfsResult<()>;
}
