use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ofs_core::errors::{OfsError, OfsResult};
use ofs_core::traits::{MaterializationEngine, OfflineStore, OnlineStore, Registry, RetrievalJob};
use ofs_core::types::{EntityKey, FeatureView, OnlineWriteRecord, RepoConfig};
use std::collections::HashMap;
use std::sync::Arc;

/// Default implementation of the `MaterializationEngine` trait.
///
/// Reads features from an offline store and writes them to an online store.
/// Uses DuckDB to execute the SQL queries returned by the offline store.
pub struct DefaultMaterializationEngine {
    registry: Arc<dyn Registry>,
    offline_store: Arc<dyn OfflineStore>,
    online_store: Arc<dyn OnlineStore>,
    #[allow(dead_code)]
    config: RepoConfig,
    duckdb_path: Option<String>,
}

impl DefaultMaterializationEngine {
    pub fn new(
        registry: Arc<dyn Registry>,
        offline_store: Arc<dyn OfflineStore>,
        online_store: Arc<dyn OnlineStore>,
        config: RepoConfig,
    ) -> Self {
        let duckdb_path = match &config.offline_store {
            ofs_core::types::OfflineStoreConfig::DuckDb { path } => path.clone(),
        };
        Self {
            registry,
            offline_store,
            online_store,
            config,
            duckdb_path,
        }
    }

    /// Materialize a single feature view between start and end dates.
    async fn materialize_feature_view(
        &self,
        fv: &FeatureView,
        project: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        full_feature_names: bool,
    ) -> OfsResult<()> {
        let RetrievalJob { query: sql, .. } =
            self.offline_store.pull_features(fv, start, end).await?;

        let results = self.execute_query(&sql)?;

        // Convert results to OnlineWriteRecords
        if results.is_empty() {
            return Ok(());
        }

        // Build entity key columns from the feature view's entities
        let entity_key_cols: Vec<String> = fv.entities.clone();
        let _feature_names: Vec<String> = fv.features.iter().map(|f| f.name.clone()).collect();

        let mut write_records = Vec::new();

        for row in &results {
            let mut entity_key = EntityKey::new(entity_key_cols.clone());

            // Collect entity values and feature values from the row
            let mut values = HashMap::new();
            let mut row_ts: Option<DateTime<Utc>> = None;

            for (col_name, col_val) in row {
                if col_name == "event_timestamp" || col_name == "event_ts" {
                    if let Ok(ts) = col_val.parse::<DateTime<Utc>>() {
                        row_ts = Some(ts);
                    }
                } else if entity_key_cols.contains(col_name) {
                    // This is an entity key column - store value in entity_key
                    let idx = entity_key_cols.iter().position(|c| c == col_name).unwrap();
                    while entity_key.entity_values.len() <= idx {
                        entity_key.entity_values.push(Vec::new());
                        entity_key
                            .value_types
                            .push(ofs_core::value_type::ValueType::String);
                    }
                    entity_key.entity_values[idx] = col_val.as_bytes().to_vec();
                } else {
                    // This is a feature value
                    let clean_name = if full_feature_names {
                        col_name.clone()
                    } else {
                        // Strip {fv_name}__ prefix if present
                        let prefix = format!("{}__", fv.name);
                        col_name
                            .strip_prefix(&prefix)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| col_name.clone())
                    };
                    values.insert(clean_name, col_val.as_bytes().to_vec());
                }
            }

            let timestamp = row_ts.unwrap_or(end);
            write_records.push(OnlineWriteRecord {
                entity_key,
                values,
                timestamp,
                feature_view_name: fv.name.clone(),
            });
        }

        // Write to online store in batches
        for chunk in write_records.chunks(100) {
            self.online_store
                .online_write_batch(chunk.to_vec(), project)
                .await?;
        }

        // Record the materialization interval
        self.registry
            .apply_materialization(&fv.name, project, start, end)
            .await?;

        Ok(())
    }

    /// Execute a DuckDB SQL query and return results as rows of string key-value pairs.
    fn execute_query(&self, sql: &str) -> OfsResult<Vec<Vec<(String, String)>>> {
        let conn = match &self.duckdb_path {
            Some(path) => {
                duckdb::Connection::open(path).map_err(|e| OfsError::DuckDb(e.to_string()))?
            }
            None => {
                duckdb::Connection::open_in_memory().map_err(|e| OfsError::DuckDb(e.to_string()))?
            }
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| OfsError::DuckDb(e.to_string()))?;

        // Execute first, then get column metadata from rows.as_ref()
        let mut rows = stmt
            .query([])
            .map_err(|e| OfsError::DuckDb(e.to_string()))?;

        let stmt_ref = rows.as_ref().expect("statement not available after query");
        let col_count = stmt_ref.column_count();
        let col_names: Vec<String> = (0..col_count)
            .filter_map(|i| stmt_ref.column_name(i).ok().cloned())
            .collect();

        let mut results = Vec::new();
        while let Some(row) = rows.next().map_err(|e| OfsError::DuckDb(e.to_string()))? {
            let mut fields = Vec::new();
            for i in 0..col_count {
                let val: duckdb::types::Value = row.get(i).unwrap_or(duckdb::types::Value::Null);
                let s = value_to_string(&val);
                let name = col_names
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i));
                fields.push((name, s));
            }
            results.push(fields);
        }

        Ok(results)
    }
}

fn value_to_string(v: &duckdb::types::Value) -> String {
    use duckdb::types::Value;
    match v {
        Value::Null => String::new(),
        Value::Boolean(b) => b.to_string(),
        Value::TinyInt(i) => i.to_string(),
        Value::SmallInt(i) => i.to_string(),
        Value::Int(i) => i.to_string(),
        Value::BigInt(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(f) => f.to_string(),
        Value::Text(s) => s.clone(),
        Value::Blob(b) => String::from_utf8_lossy(b).to_string(),
        Value::Date32(d) => d.to_string(),
        Value::Time64(..) => String::new(),
        Value::Timestamp(_unit, ts) => {
            let micros = match _unit {
                duckdb::types::TimeUnit::Second => ts.checked_mul(1_000_000),
                duckdb::types::TimeUnit::Millisecond => ts.checked_mul(1_000),
                duckdb::types::TimeUnit::Microsecond => Some(*ts),
                duckdb::types::TimeUnit::Nanosecond => ts.checked_div(1_000),
            };
            micros
                .and_then(DateTime::from_timestamp_micros)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }
        _ => String::new(),
    }
}

#[async_trait]
impl MaterializationEngine for DefaultMaterializationEngine {
    async fn materialize(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        feature_views: Option<Vec<String>>,
        project: &str,
        full_feature_names: bool,
    ) -> OfsResult<()> {
        let fv_list = self.registry.list_feature_views(project).await?;

        let target_fvs: Vec<&FeatureView> = match &feature_views {
            Some(names) => fv_list
                .iter()
                .filter(|fv| names.contains(&fv.name))
                .collect(),
            None => fv_list.iter().collect(),
        };

        if target_fvs.is_empty() {
            return Err(OfsError::NotFound(
                "No feature views found to materialize".to_string(),
            ));
        }

        for fv in target_fvs {
            tracing::info!(
                "Materializing feature view '{}' from {} to {}",
                fv.name,
                start_date,
                end_date
            );

            self.materialize_feature_view(fv, project, start_date, end_date, full_feature_names)
                .await?;
        }

        self.registry.commit().await?;

        Ok(())
    }

    async fn materialize_incremental(
        &self,
        end_date: DateTime<Utc>,
        feature_views: Option<Vec<String>>,
        project: &str,
        full_feature_names: bool,
    ) -> OfsResult<()> {
        let fv_list = self.registry.list_feature_views(project).await?;

        let target_fvs: Vec<&FeatureView> = match &feature_views {
            Some(names) => fv_list
                .iter()
                .filter(|fv| names.contains(&fv.name))
                .collect(),
            None => fv_list.iter().collect(),
        };

        if target_fvs.is_empty() {
            return Err(OfsError::NotFound(
                "No feature views found to materialize".to_string(),
            ));
        }

        for fv in target_fvs {
            let intervals = self
                .registry
                .get_materialization_intervals(&fv.name, project)
                .await?;

            let start_date = intervals
                .iter()
                .map(|(_, end)| *end)
                .max()
                .or_else(|| {
                    fv.ttl
                        .map(|ttl| end_date - chrono::Duration::from_std(ttl).unwrap())
                })
                .unwrap_or_else(|| {
                    // Default: look back 1 day if no prior materialization
                    end_date - chrono::Duration::hours(24)
                });

            if start_date >= end_date {
                tracing::info!(
                    "Feature view '{}' is already up-to-date (last materialized at {})",
                    fv.name,
                    start_date
                );
                continue;
            }

            tracing::info!(
                "Incrementally materializing feature view '{}' from {} to {}",
                fv.name,
                start_date,
                end_date
            );

            self.materialize_feature_view(fv, project, start_date, end_date, full_feature_names)
                .await?;
        }

        self.registry.commit().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ofs_core::traits::{
        EntityDataFrame, OnlineReadResponse, OnlineResponseMetadata, RetrievalJob,
    };
    use ofs_core::types::*;
    use ofs_core::value_type::ValueType;

    struct MockRegistry {
        feature_views: Vec<FeatureView>,
        intervals: Vec<(String, Vec<(DateTime<Utc>, DateTime<Utc>)>)>,
    }

    impl MockRegistry {
        fn new() -> Self {
            Self {
                feature_views: Vec::new(),
                intervals: Vec::new(),
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
            fv_name: &str,
            _project: &str,
            _start: DateTime<Utc>,
            _end: DateTime<Utc>,
        ) -> OfsResult<()> {
            // Store intervals for mock queries
            let _ = fv_name;
            Ok(())
        }
        async fn get_materialization_intervals(
            &self,
            fv_name: &str,
            _project: &str,
        ) -> OfsResult<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
            Ok(self
                .intervals
                .iter()
                .find(|(name, _)| name == fv_name)
                .map(|(_, intervals)| intervals.clone())
                .unwrap_or_default())
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
        async fn create_backfill_job(&self, _job: &BackfillJob) -> OfsResult<()> {
            Ok(())
        }
        async fn get_backfill_job(&self, _job_id: &str) -> OfsResult<Option<BackfillJob>> {
            Ok(None)
        }
        async fn list_backfill_jobs(&self, _project: &str) -> OfsResult<Vec<BackfillJob>> {
            Ok(Vec::new())
        }
        async fn update_backfill_job(&self, _job: &BackfillJob) -> OfsResult<()> {
            Ok(())
        }
    }

    struct MockOfflineStore;

    #[async_trait]
    impl OfflineStore for MockOfflineStore {
        async fn get_historical_features(
            &self,
            _entity_df: EntityDataFrame,
            _features: Vec<FeatureViewWithProjection>,
            _config: &RepoConfig,
        ) -> OfsResult<RetrievalJob> {
            Ok(RetrievalJob {
                query: String::new(),
                schema_fields: Vec::new(),
            })
        }

        async fn pull_features(
            &self,
            _feature_view: &FeatureView,
            _start_date: DateTime<Utc>,
            _end_date: DateTime<Utc>,
        ) -> OfsResult<RetrievalJob> {
            Ok(RetrievalJob {
                query: "SELECT 1 WHERE 1=0".to_string(),
                schema_fields: Vec::new(),
            })
        }
        async fn purge_offline_data(
            &self,
            _feature_view: &FeatureView,
            _project: &str,
            _cutoff: DateTime<Utc>,
        ) -> OfsResult<u64> {
            Ok(0)
        }
    }

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
                    feature_names: Vec::new(),
                },
                results: Vec::new(),
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

    #[tokio::test]
    async fn test_materialize_no_feature_views() {
        let registry = Arc::new(MockRegistry::new()) as Arc<dyn Registry>;
        let offline = Arc::new(MockOfflineStore) as Arc<dyn OfflineStore>;
        let online = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;

        let engine =
            DefaultMaterializationEngine::new(registry, offline, online, RepoConfig::default());

        let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let result = engine.materialize(start, end, None, "default", false).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No feature views found")
        );
    }

    #[tokio::test]
    async fn test_materialize_with_feature_views() {
        let mut registry = MockRegistry::new();
        let mut fv = FeatureView::new("driver_stats");
        fv.features
            .push(Feature::new("conv_rate", ValueType::Double));
        fv.batch_source = Some(DataSource::new(
            "source",
            DataSourceOptions::File {
                path: "data/drivers.parquet".to_string(),
                file_format: FileFormat::Parquet,
                s3_endpoint_override: None,
            },
        ));
        registry.feature_views.push(fv);

        let registry = Arc::new(registry) as Arc<dyn Registry>;
        let offline = Arc::new(MockOfflineStore) as Arc<dyn OfflineStore>;
        let online = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;

        let engine =
            DefaultMaterializationEngine::new(registry, offline, online, RepoConfig::default());

        let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // Should succeed and call pull_features + write
        let result = engine
            .materialize(
                start,
                end,
                Some(vec!["driver_stats".to_string()]),
                "default",
                false,
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_materialize_incremental_no_prior() {
        let mut registry = MockRegistry::new();
        let mut fv = FeatureView::new("driver_stats");
        fv.features
            .push(Feature::new("conv_rate", ValueType::Double));
        fv.batch_source = Some(DataSource::new(
            "source",
            DataSourceOptions::File {
                path: "data/drivers.parquet".to_string(),
                file_format: FileFormat::Parquet,
                s3_endpoint_override: None,
            },
        ));
        registry.feature_views.push(fv);

        let registry = Arc::new(registry) as Arc<dyn Registry>;
        let offline = Arc::new(MockOfflineStore) as Arc<dyn OfflineStore>;
        let online = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;

        let engine =
            DefaultMaterializationEngine::new(registry, offline, online, RepoConfig::default());

        let end = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let result = engine
            .materialize_incremental(end, None, "default", false)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_materialize_incremental_with_prior() {
        let mut registry = MockRegistry::new();
        let mut fv = FeatureView::new("driver_stats");
        fv.features
            .push(Feature::new("conv_rate", ValueType::Double));
        fv.batch_source = Some(DataSource::new(
            "source",
            DataSourceOptions::File {
                path: "data/drivers.parquet".to_string(),
                file_format: FileFormat::Parquet,
                s3_endpoint_override: None,
            },
        ));
        registry.feature_views.push(fv);

        let prior_start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let prior_end = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        registry
            .intervals
            .push(("driver_stats".to_string(), vec![(prior_start, prior_end)]));

        let registry = Arc::new(registry) as Arc<dyn Registry>;
        let offline = Arc::new(MockOfflineStore) as Arc<dyn OfflineStore>;
        let online = Arc::new(MockOnlineStore) as Arc<dyn OnlineStore>;

        let engine =
            DefaultMaterializationEngine::new(registry, offline, online, RepoConfig::default());

        let end = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let result = engine
            .materialize_incremental(end, None, "default", false)
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_value_to_string() {
        assert_eq!(value_to_string(&duckdb::types::Value::Null), "");
        assert_eq!(value_to_string(&duckdb::types::Value::Int(42)), "42");
        assert_eq!(
            value_to_string(&duckdb::types::Value::Text("hello".to_string())),
            "hello"
        );
        assert_eq!(value_to_string(&duckdb::types::Value::Double(3.14)), "3.14");
    }
}
