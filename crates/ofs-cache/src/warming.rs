use ofs_core::errors::OfsResult;
use ofs_core::traits::OnlineStore;
use ofs_core::types::{EntityKey, FeatureViewWithProjection};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

use crate::traits::{CacheKey, CachedValue, FeatureCache};

/// A warm entry specifying which feature + entity to pre-cache.
#[derive(Debug, Clone)]
pub struct WarmEntry {
    pub project: String,
    pub feature_view: FeatureViewWithProjection,
    pub entity_keys: Vec<EntityKey>,
}

/// Cache warming background task.
///
/// Periodically pre-populates the cache with frequently accessed features.
pub struct CacheWarmer {
    online_store: Arc<dyn OnlineStore>,
    cache: Arc<dyn FeatureCache>,
    entries: Vec<WarmEntry>,
    interval: Duration,
    shutdown: Arc<Notify>,
}

impl CacheWarmer {
    pub fn new(
        online_store: Arc<dyn OnlineStore>,
        cache: Arc<dyn FeatureCache>,
        interval: Duration,
    ) -> Self {
        Self {
            online_store,
            cache,
            entries: Vec::new(),
            interval,
            shutdown: Arc::new(Notify::new()),
        }
    }

    pub fn add_entry(&mut self, entry: WarmEntry) {
        self.entries.push(entry);
    }

    pub fn with_entries(mut self, entries: Vec<WarmEntry>) -> Self {
        self.entries = entries;
        self
    }

    /// Start the warming loop in a background task.
    pub fn start(self) -> tokio::task::JoinHandle<OfsResult<()>> {
        let shutdown = self.shutdown.clone();
        tokio::spawn(async move {
            if let Err(e) = self.do_warm().await {
                tracing::warn!("Initial cache warm failed: {}", e);
            }

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        tracing::info!("Cache warmer shutting down");
                        return Ok(());
                    }
                    _ = tokio::time::sleep(self.interval) => {
                        if let Err(e) = self.do_warm().await {
                            tracing::warn!("Cache warm cycle failed: {}", e);
                        }
                    }
                }
            }
        })
    }

    pub fn shutdown_signal(&self) -> Arc<Notify> {
        self.shutdown.clone()
    }

    async fn do_warm(&self) -> OfsResult<()> {
        for entry in &self.entries {
            if entry.entity_keys.is_empty() {
                continue;
            }

            let response = self
                .online_store
                .online_read(
                    entry.entity_keys.clone(),
                    std::slice::from_ref(&entry.feature_view),
                    &entry.project,
                )
                .await?;

            for (i, ek) in entry.entity_keys.iter().enumerate() {
                if i >= response.results.len() {
                    break;
                }
                let fv = &response.results[i];

                let ck = CacheKey {
                    project: entry.project.clone(),
                    feature_view: entry.feature_view.feature_view.name.clone(),
                    entity_key: ek.join_keys.join(":"),
                };

                let cached = CachedValue {
                    values: fv.values.clone(),
                    statuses: fv.statuses.iter().map(|s| *s as i32).collect(),
                    event_timestamps: fv
                        .event_timestamps
                        .iter()
                        .map(|ts| ts.map(|t| t.to_rfc3339()))
                        .collect(),
                    cached_at: chrono::Utc::now().to_rfc3339(),
                };

                if let Err(e) = self.cache.set(ck, cached).await {
                    tracing::warn!("Failed to warm cache entry: {}", e);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use ofs_core::traits::{
        FeatureVector, FieldStatus, OnlineReadResponse, OnlineResponseMetadata,
    };
    use ofs_core::types::*;
    use std::collections::HashMap;

    struct MockOnlineStore;
    #[async_trait]
    impl OnlineStore for MockOnlineStore {
        async fn online_read(
            &self,
            _entity_keys: Vec<EntityKey>,
            _features: &[FeatureViewWithProjection],
            _project: &str,
        ) -> OfsResult<OnlineReadResponse> {
            Ok(OnlineReadResponse {
                metadata: OnlineResponseMetadata {
                    feature_names: vec!["driver_stats__conv_rate".to_string()],
                },
                results: vec![FeatureVector {
                    values: vec![b"0.85".to_vec()],
                    statuses: vec![FieldStatus::Present],
                    event_timestamps: vec![Some(
                        chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                            .unwrap()
                            .with_timezone(&chrono::Utc),
                    )],
                }],
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

    struct MockCache;
    #[async_trait]
    impl FeatureCache for MockCache {
        async fn get(&self, _key: &CacheKey) -> OfsResult<Option<CachedValue>> {
            Ok(None)
        }
        async fn set(&self, _key: CacheKey, _value: CachedValue) -> OfsResult<()> {
            Ok(())
        }
        async fn invalidate(&self, _key: &CacheKey) -> OfsResult<()> {
            Ok(())
        }
        async fn clear(&self) -> OfsResult<()> {
            Ok(())
        }
    }

    fn make_fvp(name: &str) -> FeatureViewWithProjection {
        let fv = FeatureView::new(name);
        FeatureViewWithProjection {
            feature_view: fv,
            projection: FeatureViewProjection {
                feature_view_name: name.to_string(),
                feature_view_name_alias: None,
                feature_columns: Vec::new(),
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
    async fn test_create_warmer() {
        let online = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;
        let cache = Arc::new(MockCache) as Arc<dyn FeatureCache>;
        let warmer = CacheWarmer::new(online, cache, Duration::from_secs(60));
        assert!(warmer.entries.is_empty());
    }

    #[tokio::test]
    async fn test_warm_with_entry() {
        let online = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;
        let cache = Arc::new(MockCache) as Arc<dyn FeatureCache>;
        let mut warmer = CacheWarmer::new(online, cache, Duration::from_secs(60));

        let entry = WarmEntry {
            project: "default".to_string(),
            feature_view: make_fvp("driver_stats"),
            entity_keys: vec![EntityKey::new(vec!["driver-1".to_string()])],
        };
        warmer.add_entry(entry);

        // do_warm should not error
        let result = warmer.do_warm().await;
        assert!(result.is_ok());
    }
}
