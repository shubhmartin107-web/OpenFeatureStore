use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::errors::OfsResult;
use crate::types::{
    BackfillJob, DataSource, Entity, EntityKey, FeatureService, FeatureView,
    FeatureViewWithProjection, OnDemandFeatureView, OnlineWriteRecord, RepoConfig,
};

/// Registry trait for storing and retrieving feature store metadata.
#[async_trait]
pub trait Registry: Send + Sync {
    async fn apply_entity(&self, entity: &Entity, project: &str) -> OfsResult<()>;
    async fn get_entity(&self, name: &str, project: &str) -> OfsResult<Option<Entity>>;
    async fn list_entities(&self, project: &str) -> OfsResult<Vec<Entity>>;
    async fn delete_entity(&self, name: &str, project: &str) -> OfsResult<()>;

    async fn apply_feature_view(&self, fv: &FeatureView, project: &str) -> OfsResult<()>;
    async fn get_feature_view(&self, name: &str, project: &str) -> OfsResult<Option<FeatureView>>;
    async fn list_feature_views(&self, project: &str) -> OfsResult<Vec<FeatureView>>;
    async fn delete_feature_view(&self, name: &str, project: &str) -> OfsResult<()>;

    async fn apply_feature_service(&self, fs: &FeatureService, project: &str) -> OfsResult<()>;
    async fn get_feature_service(
        &self,
        name: &str,
        project: &str,
    ) -> OfsResult<Option<FeatureService>>;
    async fn list_feature_services(&self, project: &str) -> OfsResult<Vec<FeatureService>>;
    async fn delete_feature_service(&self, name: &str, project: &str) -> OfsResult<()>;

    async fn apply_data_source(&self, ds: &DataSource, project: &str) -> OfsResult<()>;
    async fn get_data_source(&self, name: &str, project: &str) -> OfsResult<Option<DataSource>>;
    async fn list_data_sources(&self, project: &str) -> OfsResult<Vec<DataSource>>;
    async fn delete_data_source(&self, name: &str, project: &str) -> OfsResult<()>;

    async fn apply_on_demand_feature_view(
        &self,
        odfv: &OnDemandFeatureView,
        project: &str,
    ) -> OfsResult<()>;
    async fn list_on_demand_feature_views(
        &self,
        project: &str,
    ) -> OfsResult<Vec<OnDemandFeatureView>>;

    async fn apply_materialization(
        &self,
        fv_name: &str,
        project: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> OfsResult<()>;

    async fn get_materialization_intervals(
        &self,
        fv_name: &str,
        project: &str,
    ) -> OfsResult<Vec<(DateTime<Utc>, DateTime<Utc>)>>;

    /// Remove specific materialization intervals from a feature view.
    async fn remove_materialization_intervals(
        &self,
        fv_name: &str,
        project: &str,
        intervals: &[(DateTime<Utc>, DateTime<Utc>)],
    ) -> OfsResult<()>;

    async fn commit(&self) -> OfsResult<()>;

    async fn create_backfill_job(&self, job: &BackfillJob) -> OfsResult<()>;
    async fn get_backfill_job(&self, job_id: &str) -> OfsResult<Option<BackfillJob>>;
    async fn list_backfill_jobs(&self, project: &str) -> OfsResult<Vec<BackfillJob>>;
    async fn update_backfill_job(&self, job: &BackfillJob) -> OfsResult<()>;
}

/// Retrieval job returned by the offline store's `get_historical_features`.
pub struct RetrievalJob {
    pub query: String,
    pub schema_fields: Vec<String>,
}

/// Offline store trait for historical feature retrieval.
#[async_trait]
pub trait OfflineStore: Send + Sync {
    async fn get_historical_features(
        &self,
        entity_df: EntityDataFrame,
        features: Vec<FeatureViewWithProjection>,
        config: &RepoConfig,
    ) -> OfsResult<RetrievalJob>;

    async fn pull_features(
        &self,
        feature_view: &FeatureView,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> OfsResult<RetrievalJob>;

    /// Purge feature data older than the given cutoff for a feature view.
    /// Returns the number of rows purged.
    async fn purge_offline_data(
        &self,
        feature_view: &FeatureView,
        project: &str,
        cutoff: DateTime<Utc>,
    ) -> OfsResult<u64>;
}

/// An entity dataframe for historical feature retrieval.
#[derive(Debug, Clone)]
pub struct EntityDataFrame {
    pub columns: Vec<String>,
    pub arrow_data: Vec<u8>,
    pub num_rows: usize,
    pub timestamp_column: String,
    pub entity_key_columns: Vec<String>,
}

/// Response from an online store read.
#[derive(Debug, Clone)]
pub struct OnlineReadResponse {
    pub metadata: OnlineResponseMetadata,
    pub results: Vec<FeatureVector>,
}

#[derive(Debug, Clone)]
pub struct OnlineResponseMetadata {
    pub feature_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FeatureVector {
    pub values: Vec<Vec<u8>>,
    pub statuses: Vec<FieldStatus>,
    pub event_timestamps: Vec<Option<DateTime<Utc>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldStatus {
    Invalid = 0,
    Present = 1,
    NullValue = 2,
    NotFound = 3,
    OutsideMaxAge = 4,
}

#[async_trait]
pub trait OnlineStore: Send + Sync {
    async fn online_read(
        &self,
        entity_keys: Vec<EntityKey>,
        features: &[FeatureViewWithProjection],
        project: &str,
    ) -> OfsResult<OnlineReadResponse>;

    async fn online_write_batch(
        &self,
        data: Vec<OnlineWriteRecord>,
        project: &str,
    ) -> OfsResult<()>;

    async fn update(
        &self,
        tables_to_keep: Vec<String>,
        tables_to_delete: Vec<String>,
    ) -> OfsResult<()>;

    /// Purge online feature entries older than the given cutoff for a feature view.
    /// Returns the number of entries purged.
    async fn purge_expired(
        &self,
        feature_view_name: &str,
        project: &str,
        cutoff: DateTime<Utc>,
    ) -> OfsResult<u64>;

    async fn teardown(&self) -> OfsResult<()>;
}

#[async_trait]
pub trait MaterializationEngine: Send + Sync {
    async fn materialize(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        feature_views: Option<Vec<String>>,
        project: &str,
        full_feature_names: bool,
    ) -> OfsResult<()>;

    async fn materialize_incremental(
        &self,
        end_date: DateTime<Utc>,
        feature_views: Option<Vec<String>>,
        project: &str,
        full_feature_names: bool,
    ) -> OfsResult<()>;
}
