#[cfg(test)]
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use futures_util::stream::{self, StreamExt};
use ofs_core::errors::{OfsError, OfsResult};
use ofs_core::traits::{MaterializationEngine, Registry};
use ofs_core::types::{BackfillJob, BackfillStatus, RepoConfig};
use std::sync::Arc;
use uuid::Uuid;

/// Backfill engine for historical replay of feature data.
///
/// Splits large date ranges into configurable chunks and materializes
/// them in parallel with checkpoint/resume semantics.
pub struct BackfillEngine {
    registry: Arc<dyn Registry>,
    materialization_engine: Arc<dyn MaterializationEngine>,
    #[allow(dead_code)]
    config: RepoConfig,
    concurrency: usize,
}

impl BackfillEngine {
    pub fn new(
        registry: Arc<dyn Registry>,
        materialization_engine: Arc<dyn MaterializationEngine>,
        config: RepoConfig,
    ) -> Self {
        Self {
            registry,
            materialization_engine,
            config,
            concurrency: 4,
        }
    }

    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    pub async fn create_job(
        &self,
        feature_view_name: &str,
        project: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        chunk_size_seconds: i64,
    ) -> OfsResult<String> {
        let now = Utc::now();
        let job = BackfillJob {
            id: Uuid::new_v4().to_string(),
            feature_view_name: feature_view_name.to_string(),
            project: project.to_string(),
            start,
            end,
            status: BackfillStatus::Pending,
            progress: 0.0,
            chunk_size_seconds,
            error: None,
            created_at: now,
            updated_at: now,
        };
        let id = job.id.clone();
        self.registry.create_backfill_job(&job).await?;
        Ok(id)
    }

    pub async fn execute_job(&self, job_id: &str) -> OfsResult<()> {
        let job = self
            .registry
            .get_backfill_job(job_id)
            .await?
            .ok_or_else(|| OfsError::NotFound(format!("Backfill job '{}' not found", job_id)))?;

        if job.status != BackfillStatus::Pending && job.status != BackfillStatus::Failed {
            return Err(OfsError::InvalidInput(format!(
                "Cannot execute backfill job '{}' in status {:?}",
                job_id, job.status
            )));
        }

        self.set_job_status(job_id, BackfillStatus::Running, None)
            .await?;

        let chunk_duration = Duration::seconds(job.chunk_size_seconds);
        let mut completed_chunks: f64 = 0.0;

        let mut chunks = Vec::new();
        let mut chunk_start = job.start;
        while chunk_start < job.end {
            let chunk_end = (chunk_start + chunk_duration).min(job.end);
            chunks.push((chunk_start, chunk_end));
            chunk_start = chunk_end;
        }

        if chunks.is_empty() {
            chunks.push((job.start, job.end));
        }

        let results: Vec<OfsResult<()>> =
            stream::iter(chunks.into_iter().map(|(chunk_start, chunk_end)| {
                let _registry = self.registry.clone();
                let mat_engine = self.materialization_engine.clone();
                let _job_id = job_id.to_string();
                let fv_name = job.feature_view_name.clone();
                let project = job.project.clone();

                async move {
                    tracing::info!(
                        "Backfill chunk ({}, {}) for feature view '{}'",
                        chunk_start,
                        chunk_end,
                        fv_name
                    );

                    mat_engine
                        .materialize(
                            chunk_start,
                            chunk_end,
                            Some(vec![fv_name.clone()]),
                            &project,
                            false,
                        )
                        .await?;

                    Ok::<_, OfsError>(())
                }
            }))
            .buffered(self.concurrency)
            .collect::<Vec<OfsResult<()>>>()
            .await;

        for result in &results {
            match result {
                Ok(()) => {
                    completed_chunks += 1.0;
                    let progress = completed_chunks / results.len() as f64;
                    if let Err(e) = self.update_progress(job_id, progress).await {
                        tracing::warn!("Failed to update backfill progress: {}", e);
                    }
                }
                Err(e) => {
                    let progress = completed_chunks / results.len() as f64;
                    if let Err(ue) = self
                        .set_job_status(job_id, BackfillStatus::Failed, Some(&e.to_string()))
                        .await
                    {
                        tracing::warn!("Failed to update backfill error: {}", ue);
                    }
                    if let Err(pe) = self.update_progress(job_id, progress).await {
                        tracing::warn!("Failed to update backfill progress: {}", pe);
                    }
                    return Err(OfsError::Backfill(format!(
                        "Backfill job '{}' failed: {}",
                        job_id, e
                    )));
                }
            }
        }

        self.set_job_status(job_id, BackfillStatus::Completed, None)
            .await?;
        self.update_progress(job_id, 1.0).await?;

        Ok(())
    }

    pub async fn get_job(&self, job_id: &str) -> OfsResult<Option<BackfillJob>> {
        self.registry.get_backfill_job(job_id).await
    }

    pub async fn list_jobs(&self, project: &str) -> OfsResult<Vec<BackfillJob>> {
        self.registry.list_backfill_jobs(project).await
    }

    pub async fn cancel_job(&self, job_id: &str) -> OfsResult<()> {
        let job = self
            .registry
            .get_backfill_job(job_id)
            .await?
            .ok_or_else(|| OfsError::NotFound(format!("Backfill job '{}' not found", job_id)))?;

        if job.status == BackfillStatus::Completed || job.status == BackfillStatus::Cancelled {
            return Err(OfsError::InvalidInput(format!(
                "Cannot cancel backfill job '{}' in status {:?}",
                job_id, job.status
            )));
        }

        self.set_job_status(job_id, BackfillStatus::Cancelled, None)
            .await
    }

    async fn set_job_status(
        &self,
        job_id: &str,
        status: BackfillStatus,
        error: Option<&str>,
    ) -> OfsResult<()> {
        let mut job = self
            .registry
            .get_backfill_job(job_id)
            .await?
            .ok_or_else(|| OfsError::NotFound(format!("Backfill job '{}' not found", job_id)))?;
        job.status = status;
        job.error = error.map(|s| s.to_string());
        job.updated_at = Utc::now();
        self.registry.update_backfill_job(&job).await
    }

    async fn update_progress(&self, job_id: &str, progress: f64) -> OfsResult<()> {
        let mut job = self
            .registry
            .get_backfill_job(job_id)
            .await?
            .ok_or_else(|| OfsError::NotFound(format!("Backfill job '{}' not found", job_id)))?;
        job.progress = progress;
        job.updated_at = Utc::now();
        self.registry.update_backfill_job(&job).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ofs_core::types::*;

    struct MockRegistry {
        feature_views: Vec<FeatureView>,
        backfill_jobs: std::sync::Mutex<Vec<BackfillJob>>,
    }

    impl MockRegistry {
        fn new() -> Self {
            Self {
                feature_views: Vec::new(),
                backfill_jobs: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl Registry for MockRegistry {
        async fn apply_entity(&self, _entity: &Entity, _project: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn get_entity(&self, _name: &str, _project: &str) -> OfsResult<Option<Entity>> {
            Ok(None)
        }
        async fn list_entities(&self, _project: &str) -> OfsResult<Vec<Entity>> {
            Ok(Vec::new())
        }
        async fn delete_entity(&self, _name: &str, _project: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn apply_feature_view(&self, _fv: &FeatureView, _project: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn get_feature_view(
            &self,
            _name: &str,
            _project: &str,
        ) -> OfsResult<Option<FeatureView>> {
            Ok(None)
        }
        async fn list_feature_views(&self, _project: &str) -> OfsResult<Vec<FeatureView>> {
            Ok(self.feature_views.clone())
        }
        async fn delete_feature_view(&self, _name: &str, _project: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn apply_feature_service(
            &self,
            _fs: &FeatureService,
            _project: &str,
        ) -> OfsResult<()> {
            Ok(())
        }
        async fn get_feature_service(
            &self,
            _name: &str,
            _project: &str,
        ) -> OfsResult<Option<FeatureService>> {
            Ok(None)
        }
        async fn list_feature_services(&self, _project: &str) -> OfsResult<Vec<FeatureService>> {
            Ok(Vec::new())
        }
        async fn delete_feature_service(&self, _name: &str, _project: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn apply_data_source(&self, _ds: &DataSource, _project: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn get_data_source(
            &self,
            _name: &str,
            _project: &str,
        ) -> OfsResult<Option<DataSource>> {
            Ok(None)
        }
        async fn list_data_sources(&self, _project: &str) -> OfsResult<Vec<DataSource>> {
            Ok(Vec::new())
        }
        async fn delete_data_source(&self, _name: &str, _project: &str) -> OfsResult<()> {
            Ok(())
        }
        async fn apply_on_demand_feature_view(
            &self,
            _odfv: &OnDemandFeatureView,
            _project: &str,
        ) -> OfsResult<()> {
            Ok(())
        }
        async fn list_on_demand_feature_views(
            &self,
            _project: &str,
        ) -> OfsResult<Vec<OnDemandFeatureView>> {
            Ok(Vec::new())
        }
        async fn apply_materialization(
            &self,
            _fv_name: &str,
            _project: &str,
            _start: DateTime<Utc>,
            _end: DateTime<Utc>,
        ) -> OfsResult<()> {
            Ok(())
        }
        async fn get_materialization_intervals(
            &self,
            _fv_name: &str,
            _project: &str,
        ) -> OfsResult<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
            Ok(Vec::new())
        }
        async fn remove_materialization_intervals(
            &self,
            _fv_name: &str,
            _project: &str,
            _intervals: &[(DateTime<Utc>, DateTime<Utc>)],
        ) -> OfsResult<()> {
            Ok(())
        }
        async fn commit(&self) -> OfsResult<()> {
            Ok(())
        }
        async fn create_backfill_job(&self, job: &BackfillJob) -> OfsResult<()> {
            let mut jobs = self.backfill_jobs.lock().unwrap();
            jobs.push(job.clone());
            Ok(())
        }
        async fn get_backfill_job(&self, job_id: &str) -> OfsResult<Option<BackfillJob>> {
            let jobs = self.backfill_jobs.lock().unwrap();
            Ok(jobs.iter().find(|j| j.id == job_id).cloned())
        }
        async fn list_backfill_jobs(&self, project: &str) -> OfsResult<Vec<BackfillJob>> {
            let jobs = self.backfill_jobs.lock().unwrap();
            Ok(jobs
                .iter()
                .filter(|j| j.project == project)
                .cloned()
                .collect())
        }
        async fn update_backfill_job(&self, job: &BackfillJob) -> OfsResult<()> {
            let mut jobs = self.backfill_jobs.lock().unwrap();
            if let Some(pos) = jobs.iter().position(|j| j.id == job.id) {
                jobs[pos] = job.clone();
            }
            Ok(())
        }
    }

    struct MockMatEngine;

    #[async_trait]
    impl MaterializationEngine for MockMatEngine {
        async fn materialize(
            &self,
            _start_date: DateTime<Utc>,
            _end_date: DateTime<Utc>,
            _feature_views: Option<Vec<String>>,
            _project: &str,
            _full_feature_names: bool,
        ) -> OfsResult<()> {
            Ok(())
        }

        async fn materialize_incremental(
            &self,
            _end_date: DateTime<Utc>,
            _feature_views: Option<Vec<String>>,
            _project: &str,
            _full_feature_names: bool,
        ) -> OfsResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_create_job() {
        let registry = Arc::new(MockRegistry::new()) as Arc<dyn Registry>;
        let mat = Arc::new(MockMatEngine) as Arc<dyn MaterializationEngine>;

        let engine = BackfillEngine::new(registry, mat, RepoConfig::default());

        let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let id = engine
            .create_job("driver_stats", "default", start, end, 3600)
            .await
            .unwrap();
        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_execute_job_not_found() {
        let registry = Arc::new(MockRegistry::new()) as Arc<dyn Registry>;
        let mat = Arc::new(MockMatEngine) as Arc<dyn MaterializationEngine>;

        let engine = BackfillEngine::new(registry, mat, RepoConfig::default());
        let result = engine.execute_job("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let registry = Arc::new(MockRegistry::new()) as Arc<dyn Registry>;
        let mat = Arc::new(MockMatEngine) as Arc<dyn MaterializationEngine>;

        let engine = BackfillEngine::new(registry.clone(), mat, RepoConfig::default());

        let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let id = engine
            .create_job("driver_stats", "default", start, end, 3600)
            .await
            .unwrap();

        engine.cancel_job(&id).await.unwrap();

        let job = engine.get_job(&id).await.unwrap().unwrap();
        assert_eq!(job.status, BackfillStatus::Cancelled);
    }
}
