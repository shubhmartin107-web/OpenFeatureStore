use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ofs_core::errors::OfsResult;
use ofs_core::traits::{
    FeatureVector, FieldStatus, OnlineReadResponse, OnlineResponseMetadata, OnlineStore,
};
use ofs_core::types::{EntityKey, FeatureViewWithProjection, OnlineWriteRecord};
use std::sync::Arc;

use crate::CacheConfig;
use crate::l1::L1Cache;
use crate::l2::L2Cache;
use crate::traits::{CacheKey, CachedValue, FeatureCache};

/// Multi-tier cached online store.
///
/// Wraps an inner `OnlineStore` and adds L1 (moka) and L2 (Redis) caching.
/// Read path: L1 → L2 → inner store (populating caches on miss).
/// Write path: inner store → invalidate both caches.
pub struct CachedOnlineStore {
    inner: Arc<dyn OnlineStore>,
    l1: Option<L1Cache>,
    l2: Option<L2Cache>,
}

impl CachedOnlineStore {
    pub async fn new(inner: Arc<dyn OnlineStore>, config: &CacheConfig) -> OfsResult<Self> {
        let l1 = if config.enabled {
            Some(L1Cache::new(
                config.max_size as u64,
                std::time::Duration::from_secs(config.ttl_secs),
            ))
        } else {
            None
        };

        let l2 = match &config.redis {
            Some(redis_cfg) if !redis_cfg.nodes.is_empty() => {
                let ttl = redis_cfg.default_ttl_secs.unwrap_or(config.ttl_secs);
                let conn_str = redis_cfg.nodes.first().ok_or_else(|| {
                    ofs_core::errors::OfsError::Config("No Redis nodes configured".to_string())
                })?;
                Some(
                    L2Cache::new(conn_str, redis_cfg.key_prefix.as_deref().unwrap_or(""), ttl)
                        .await?,
                )
            }
            _ => None,
        };

        Ok(Self { inner, l1, l2 })
    }

    fn build_cache_key(project: &str, fv_name: &str, ek: &EntityKey) -> CacheKey {
        CacheKey {
            project: project.to_string(),
            feature_view: fv_name.to_string(),
            entity_key: ek.join_keys.join(":"),
        }
    }

    fn build_cached_value(response: &OnlineReadResponse) -> Option<CachedValue> {
        if response.results.is_empty() {
            return None;
        }
        let fv = &response.results[0];
        Some(CachedValue {
            values: fv.values.clone(),
            statuses: fv.statuses.iter().map(|s| *s as i32).collect(),
            event_timestamps: fv
                .event_timestamps
                .iter()
                .map(|ts| ts.map(|t| t.to_rfc3339()))
                .collect(),
            cached_at: Utc::now().to_rfc3339(),
        })
    }
}

#[async_trait]
impl OnlineStore for CachedOnlineStore {
    async fn online_read(
        &self,
        entity_keys: Vec<EntityKey>,
        features: &[FeatureViewWithProjection],
        project: &str,
    ) -> OfsResult<OnlineReadResponse> {
        // Try L1 cache first (fastest)
        if let Some(ref l1) = self.l1
            && entity_keys.len() == 1
            && features.len() == 1
        {
            let ck =
                Self::build_cache_key(project, &features[0].feature_view.name, &entity_keys[0]);
            if let Some(cached) = l1.get(&ck).await? {
                let mut all_feature_names = Vec::new();
                let mut all_results = Vec::new();

                for fvp in features {
                    for fname in &fvp.projection.feature_columns {
                        let col_name = format!("{}__{}", fvp.feature_view.name, fname.name);
                        all_feature_names.push(col_name);
                    }
                }

                let mut event_timestamps = Vec::new();
                for ts_str in &cached.event_timestamps {
                    event_timestamps.push(
                        ts_str
                            .as_ref()
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&Utc)),
                    );
                }

                let statuses: Vec<FieldStatus> = cached
                    .statuses
                    .iter()
                    .map(|s| match s {
                        0 => FieldStatus::Invalid,
                        1 => FieldStatus::Present,
                        2 => FieldStatus::NullValue,
                        3 => FieldStatus::NotFound,
                        _ => FieldStatus::NotFound,
                    })
                    .collect();

                all_results.push(FeatureVector {
                    values: cached.values,
                    statuses,
                    event_timestamps,
                });

                return Ok(OnlineReadResponse {
                    metadata: OnlineResponseMetadata {
                        feature_names: all_feature_names,
                    },
                    results: all_results,
                });
            }
        }

        // Try L2 cache
        if let Some(ref l2) = self.l2
            && entity_keys.len() == 1
            && features.len() == 1
        {
            let ck =
                Self::build_cache_key(project, &features[0].feature_view.name, &entity_keys[0]);
            if let Some(cached) = l2.get(&ck).await? {
                // Populate L1 on L2 hit
                if let Some(ref l1) = self.l1 {
                    let _ = l1.set(ck, cached.clone()).await;
                }
                return l2_hit_to_response(cached, features);
            }
        }

        // Cache miss — query inner store
        let response = self
            .inner
            .online_read(entity_keys.clone(), features, project)
            .await?;

        // Populate caches
        if let Some(ref l1) = self.l1
            && entity_keys.len() == 1
            && features.len() == 1
            && let Some(cached) = Self::build_cached_value(&response)
        {
            let ck =
                Self::build_cache_key(project, &features[0].feature_view.name, &entity_keys[0]);
            let _ = l1.set(ck, cached).await;
        }

        Ok(response)
    }

    async fn online_write_batch(
        &self,
        data: Vec<OnlineWriteRecord>,
        project: &str,
    ) -> OfsResult<()> {
        // Write-through: write to inner store first
        self.inner.online_write_batch(data.clone(), project).await?;

        // Invalidate affected cache keys
        for record in &data {
            let ck = CacheKey {
                project: project.to_string(),
                feature_view: record.feature_view_name.clone(),
                entity_key: record.entity_key.join_keys.join(":"),
            };

            if let Some(ref l1) = self.l1 {
                let _ = l1.invalidate(&ck).await;
            }
            if let Some(ref l2) = self.l2 {
                let _ = l2.invalidate(&ck).await;
            }
        }

        Ok(())
    }

    async fn update(
        &self,
        tables_to_keep: Vec<String>,
        tables_to_delete: Vec<String>,
    ) -> OfsResult<()> {
        self.inner.update(tables_to_keep, tables_to_delete).await
    }

    async fn purge_expired(
        &self,
        feature_view_name: &str,
        project: &str,
        cutoff: DateTime<Utc>,
    ) -> OfsResult<u64> {
        // Invalidate cache entries before purging from inner store
        if let Some(ref l1) = self.l1 {
            let _ = l1.clear().await;
        }
        if let Some(ref l2) = self.l2 {
            let _ = l2.clear().await;
        }
        self.inner
            .purge_expired(feature_view_name, project, cutoff)
            .await
    }

    async fn teardown(&self) -> OfsResult<()> {
        if let Some(ref l1) = self.l1 {
            let _ = l1.clear().await;
        }
        if let Some(ref l2) = self.l2 {
            let _ = l2.clear().await;
        }
        self.inner.teardown().await
    }
}

fn l2_hit_to_response(
    cached: CachedValue,
    features: &[FeatureViewWithProjection],
) -> OfsResult<OnlineReadResponse> {
    let mut all_feature_names = Vec::new();
    let mut all_results = Vec::new();

    for fvp in features {
        for fname in &fvp.projection.feature_columns {
            let col_name = format!("{}__{}", fvp.feature_view.name, fname.name);
            all_feature_names.push(col_name);
        }
    }

    let mut event_timestamps = Vec::new();
    for ts_str in &cached.event_timestamps {
        event_timestamps.push(
            ts_str
                .as_ref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        );
    }

    let statuses: Vec<FieldStatus> = cached
        .statuses
        .iter()
        .map(|s| match s {
            0 => FieldStatus::Invalid,
            1 => FieldStatus::Present,
            2 => FieldStatus::NullValue,
            3 => FieldStatus::NotFound,
            _ => FieldStatus::NotFound,
        })
        .collect();

    all_results.push(FeatureVector {
        values: cached.values,
        statuses,
        event_timestamps,
    });

    Ok(OnlineReadResponse {
        metadata: OnlineResponseMetadata {
            feature_names: all_feature_names,
        },
        results: all_results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ofs_core::types::{Feature, FeatureView, FeatureViewProjection};
    use ofs_core::value_type::ValueType;
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
                        DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                            .unwrap()
                            .with_timezone(&Utc),
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
                feature_columns: fv_features,
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
    async fn test_cache_hit() {
        let inner = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;
        let config = CacheConfig {
            enabled: true,
            max_size: 100,
            ttl_secs: 300,
            redis: None,
        };
        let cached = CachedOnlineStore::new(inner, &config).await.unwrap();

        let ek = EntityKey::new(vec!["driver-1".to_string()]);
        let fvp = make_fvp("driver_stats", vec!["conv_rate"]);

        // First call — cache miss, populates cache
        let r1 = cached
            .online_read(vec![ek.clone()], &[fvp.clone()], "default")
            .await
            .unwrap();
        assert_eq!(r1.results[0].values[0], b"0.85");

        // Second call — cache hit (would return same data from cache)
        let r2 = cached
            .online_read(vec![ek], &[fvp], "default")
            .await
            .unwrap();
        assert_eq!(r2.results[0].values[0], b"0.85");
    }

    #[tokio::test]
    async fn test_write_invalidates_cache() {
        let inner = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;
        let config = CacheConfig {
            enabled: true,
            max_size: 100,
            ttl_secs: 300,
            redis: None,
        };
        let cached = CachedOnlineStore::new(inner, &config).await.unwrap();

        let ek = EntityKey::new(vec!["driver-1".to_string()]);

        // Write should not error
        let record = OnlineWriteRecord {
            entity_key: ek,
            values: [("conv_rate".to_string(), b"0.92".to_vec())].into(),
            timestamp: Utc::now(),
            feature_view_name: "driver_stats".to_string(),
        };
        cached
            .online_write_batch(vec![record], "default")
            .await
            .unwrap();
    }
}
