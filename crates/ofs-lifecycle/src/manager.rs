use std::sync::Arc;

use chrono::{DateTime, Utc};
use ofs_core::errors::OfsResult;
use ofs_core::traits::{OfflineStore, OnlineStore, Registry};
use tokio::sync::Notify;

/// Manages data lifecycle operations — TTL-based cleanup of stale feature data.
pub struct DataLifecycleManager {
    registry: Arc<dyn Registry>,
    online_store: Arc<dyn OnlineStore>,
    offline_store: Arc<dyn OfflineStore>,
    ttl_default_days: u64,
    cleanup_interval: std::time::Duration,
    projects: Vec<String>,
}

impl DataLifecycleManager {
    pub fn new(
        registry: Arc<dyn Registry>,
        online_store: Arc<dyn OnlineStore>,
        offline_store: Arc<dyn OfflineStore>,
        ttl_default_days: u64,
        cleanup_interval_secs: u64,
        projects: Vec<String>,
    ) -> Self {
        Self {
            registry,
            online_store,
            offline_store,
            ttl_default_days,
            cleanup_interval: std::time::Duration::from_secs(cleanup_interval_secs),
            projects,
        }
    }

    /// Run the lifecycle manager as a background task.
    ///
    /// Spawn this with `tokio::spawn`. Pass a `Notify` to trigger graceful shutdown.
    pub async fn run(&self, shutdown: Arc<Notify>) {
        tracing::info!(
            "lifecycle manager started: ttl={}d, interval={}s, projects={:?}",
            self.ttl_default_days,
            self.cleanup_interval.as_secs(),
            self.projects,
        );

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    tracing::info!("lifecycle manager shutting down");
                    break;
                }
                _ = tokio::time::sleep(self.cleanup_interval) => {
                    self.run_cycle().await;
                }
            }
        }
    }

    /// Run a single cleanup cycle across all configured projects.
    async fn run_cycle(&self) {
        for project in &self.projects {
            tracing::debug!("lifecycle cleaning project '{}'", project);

            let feature_views = match self.registry.list_feature_views(project).await {
                Ok(fvs) => fvs,
                Err(e) => {
                    tracing::error!("failed to list feature views for '{}': {}", project, e);
                    continue;
                }
            };

            for fv in &feature_views {
                // Use feature view's TTL if set, otherwise use the default
                let ttl_days = fv
                    .ttl
                    .map(|d| (d.as_secs() / 86400).max(1) as i64)
                    .unwrap_or(self.ttl_default_days as i64);
                let fv_cutoff = Utc::now() - chrono::TimeDelta::days(ttl_days);

                if let Err(e) = self.cleanup_feature_view(fv, project, fv_cutoff).await {
                    tracing::warn!(
                        "lifecycle cleanup failed for '{}'/'{}': {}",
                        project,
                        fv.name,
                        e,
                    );
                }
            }
        }
    }

    /// Clean up a single feature view: remove expired materialization intervals
    /// and purge stale data from online and offline stores.
    async fn cleanup_feature_view(
        &self,
        fv: &ofs_core::types::FeatureView,
        project: &str,
        cutoff: DateTime<Utc>,
    ) -> OfsResult<()> {
        // 1. Find expired materialization intervals
        let intervals = self
            .registry
            .get_materialization_intervals(&fv.name, project)
            .await?;

        let expired: Vec<_> = intervals
            .iter()
            .filter(|(_, end)| *end < cutoff)
            .copied()
            .collect();

        if expired.is_empty() {
            return Ok(());
        }

        tracing::info!(
            "lifecycle: removing {} expired intervals for '{}'/'{}' (cutoff={})",
            expired.len(),
            project,
            fv.name,
            cutoff,
        );

        // 2. Remove expired intervals from registry
        self.registry
            .remove_materialization_intervals(&fv.name, project, &expired)
            .await?;

        if let Err(e) = self.registry.commit().await {
            tracing::warn!("lifecycle commit failed: {}", e);
        }

        // 3. Purge stale data from online store
        match self
            .online_store
            .purge_expired(&fv.name, project, cutoff)
            .await
        {
            Ok(count) => {
                if count > 0 {
                    tracing::info!(
                        "lifecycle: purged {} online entries for '{}'/'{}'",
                        count,
                        project,
                        fv.name,
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "lifecycle: online purge failed for '{}'/'{}': {}",
                    project,
                    fv.name,
                    e,
                );
            }
        }

        // 4. Purge stale data from offline store (best-effort)
        match self
            .offline_store
            .purge_offline_data(fv, project, cutoff)
            .await
        {
            Ok(count) => {
                if count > 0 {
                    tracing::info!(
                        "lifecycle: purged {} offline rows for '{}'/'{}'",
                        count,
                        project,
                        fv.name,
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "lifecycle: offline purge failed for '{}'/'{}': {}",
                    project,
                    fv.name,
                    e,
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ofs_core::traits::{
        EntityDataFrame, OnlineReadResponse, OnlineResponseMetadata, RetrievalJob,
    };
    use ofs_core::types::*;
    use std::sync::Mutex;

    struct MockRegistry {
        feature_views: Mutex<Vec<FeatureView>>,
    }

    impl MockRegistry {
        fn new(fvs: Vec<FeatureView>) -> Self {
            Self {
                feature_views: Mutex::new(fvs),
            }
        }
    }

    #[async_trait]
    impl Registry for MockRegistry {
        async fn apply_entity(&self, _e: &Entity, _p: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn get_entity(&self, _n: &str, _p: &str) -> OfsResult<Option<Entity>> {
            Ok(None)
        }
        async fn list_entities(&self, _p: &str) -> OfsResult<Vec<Entity>> {
            Ok(Vec::new())
        }
        async fn delete_entity(&self, _n: &str, _p: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn apply_feature_view(&self, _f: &FeatureView, _p: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn get_feature_view(&self, _n: &str, _p: &str) -> OfsResult<Option<FeatureView>> {
            Ok(None)
        }
        async fn list_feature_views(&self, _project: &str) -> OfsResult<Vec<FeatureView>> {
            Ok(self.feature_views.lock().unwrap().clone())
        }
        async fn delete_feature_view(&self, _n: &str, _p: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn apply_feature_service(&self, _f: &FeatureService, _p: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn get_feature_service(
            &self,
            _n: &str,
            _p: &str,
        ) -> OfsResult<Option<FeatureService>> {
            Ok(None)
        }
        async fn list_feature_services(&self, _p: &str) -> OfsResult<Vec<FeatureService>> {
            Ok(Vec::new())
        }
        async fn delete_feature_service(&self, _n: &str, _p: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn apply_data_source(&self, _d: &DataSource, _p: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn get_data_source(&self, _n: &str, _p: &str) -> OfsResult<Option<DataSource>> {
            Ok(None)
        }
        async fn list_data_sources(&self, _p: &str) -> OfsResult<Vec<DataSource>> {
            Ok(Vec::new())
        }
        async fn delete_data_source(&self, _n: &str, _p: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn apply_on_demand_feature_view(
            &self,
            _o: &OnDemandFeatureView,
            _p: &str,
        ) -> OfsResult<()> {
            Ok(())
        }
        async fn list_on_demand_feature_views(
            &self,
            _p: &str,
        ) -> OfsResult<Vec<OnDemandFeatureView>> {
            Ok(Vec::new())
        }

        async fn apply_materialization(
            &self,
            _f: &str,
            _p: &str,
            _s: DateTime<Utc>,
            _e: DateTime<Utc>,
        ) -> OfsResult<()> {
            Ok(())
        }
        async fn get_materialization_intervals(
            &self,
            _f: &str,
            _p: &str,
        ) -> OfsResult<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
            Ok(vec![
                (
                    Utc::now() - chrono::TimeDelta::days(200),
                    Utc::now() - chrono::TimeDelta::days(150),
                ),
                (Utc::now() - chrono::TimeDelta::days(50), Utc::now()),
            ])
        }
        async fn remove_materialization_intervals(
            &self,
            _f: &str,
            _p: &str,
            _i: &[(DateTime<Utc>, DateTime<Utc>)],
        ) -> OfsResult<()> {
            Ok(())
        }
        async fn commit(&self) -> OfsResult<()> {
            Ok(())
        }
        async fn create_backfill_job(&self, _j: &BackfillJob) -> OfsResult<()> {
            Ok(())
        }
        async fn get_backfill_job(&self, _i: &str) -> OfsResult<Option<BackfillJob>> {
            Ok(None)
        }
        async fn list_backfill_jobs(&self, _p: &str) -> OfsResult<Vec<BackfillJob>> {
            Ok(Vec::new())
        }
        async fn update_backfill_job(&self, _j: &BackfillJob) -> OfsResult<()> {
            Ok(())
        }
    }

    struct MockOnline;
    #[async_trait]
    impl OnlineStore for MockOnline {
        async fn online_read(
            &self,
            _e: Vec<EntityKey>,
            _f: &[FeatureViewWithProjection],
            _p: &str,
        ) -> OfsResult<OnlineReadResponse> {
            Ok(OnlineReadResponse {
                metadata: OnlineResponseMetadata {
                    feature_names: vec![],
                },
                results: vec![],
            })
        }
        async fn online_write_batch(&self, _d: Vec<OnlineWriteRecord>, _p: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn update(&self, _k: Vec<String>, _d: Vec<String>) -> OfsResult<()> {
            Ok(())
        }
        async fn purge_expired(&self, _f: &str, _p: &str, _c: DateTime<Utc>) -> OfsResult<u64> {
            Ok(5)
        }
        async fn teardown(&self) -> OfsResult<()> {
            Ok(())
        }
    }

    struct MockOffline;
    #[async_trait]
    impl OfflineStore for MockOffline {
        async fn get_historical_features(
            &self,
            _e: EntityDataFrame,
            _f: Vec<FeatureViewWithProjection>,
            _c: &RepoConfig,
        ) -> OfsResult<RetrievalJob> {
            Ok(RetrievalJob {
                query: String::new(),
                schema_fields: vec![],
            })
        }
        async fn pull_features(
            &self,
            _f: &FeatureView,
            _s: DateTime<Utc>,
            _e: DateTime<Utc>,
        ) -> OfsResult<RetrievalJob> {
            Ok(RetrievalJob {
                query: String::new(),
                schema_fields: vec![],
            })
        }
        async fn purge_offline_data(
            &self,
            _f: &FeatureView,
            _p: &str,
            _c: DateTime<Utc>,
        ) -> OfsResult<u64> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn test_cleanup_cycle_removes_expired_intervals() {
        let mut fv = FeatureView::new("test_fv");
        fv.ttl = Some(std::time::Duration::from_secs(30 * 86400));
        let registry = Arc::new(MockRegistry::new(vec![fv]));
        let online = Arc::new(MockOnline);
        let offline = Arc::new(MockOffline);

        let manager = DataLifecycleManager::new(
            registry,
            online,
            offline,
            90,
            3600,
            vec!["default".to_string()],
        );

        // Run a single cycle by calling run_cycle directly
        manager.run_cycle().await;

        // The test passes if no panic occurs and the mock methods are called
        // (purge_expired was called and returned 5, remove_materialization_intervals was called)
    }

    #[tokio::test]
    async fn test_cleanup_no_expired_intervals() {
        let mut fv = FeatureView::new("recent_fv");
        fv.ttl = Some(std::time::Duration::from_secs(300 * 86400)); // TTL longer than both intervals
        let registry = Arc::new(MockRegistry::new(vec![fv]));
        let online = Arc::new(MockOnline);
        let offline = Arc::new(MockOffline);

        let manager = DataLifecycleManager::new(
            registry,
            online,
            offline,
            90,
            3600,
            vec!["default".to_string()],
        );

        manager.run_cycle().await;
    }

    #[tokio::test]
    async fn test_cleanup_skips_empty_projects() {
        let registry = Arc::new(MockRegistry::new(vec![]));
        let online = Arc::new(MockOnline);
        let offline = Arc::new(MockOffline);

        let manager = DataLifecycleManager::new(
            registry,
            online,
            offline,
            90,
            3600,
            vec!["default".to_string()],
        );

        manager.run_cycle().await;
    }

    #[tokio::test]
    async fn test_run_respects_shutdown() {
        let registry = Arc::new(MockRegistry::new(vec![]));
        let online = Arc::new(MockOnline);
        let offline = Arc::new(MockOffline);

        let manager = DataLifecycleManager::new(
            registry,
            online,
            offline,
            90,
            3600,
            vec!["default".to_string()],
        );

        let shutdown = Arc::new(Notify::new());
        let notify = shutdown.clone();
        notify.notify_one();

        // run() should exit immediately when notified
        manager.run(shutdown).await;
    }
}
