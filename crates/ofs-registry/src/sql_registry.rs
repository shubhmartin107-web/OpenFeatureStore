use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ofs_core::errors::{OfsError, OfsResult};
use ofs_core::traits::Registry;
use ofs_core::types::{
    BackfillJob, BackfillStatus, DataSource, Entity, FeatureService, FeatureView,
    OnDemandFeatureView,
};
use ofs_proto::feast::core as proto;
use sqlx::SqlitePool;

use crate::conversion;
use crate::schema;

/// A SQL-backed registry that stores Feast metadata in SQLite.
pub struct SqlRegistry {
    pool: SqlitePool,
}

impl SqlRegistry {
    /// Create a new `SqlRegistry`, running migrations if needed.
    pub async fn new(pool: SqlitePool) -> OfsResult<Self> {
        schema::run_migrations(&pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(Self { pool })
    }

    /// Create an in-memory SQLite registry (for testing).
    pub async fn in_memory() -> OfsResult<Self> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Self::new(pool).await
    }

    /// Load the full registry blob for a project, deserialize it.
    async fn load_registry(&self, project: &str) -> OfsResult<proto::Registry> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT serialized_registry FROM project_registries WHERE project_name = ?",
        )
        .bind(project)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;

        match row {
            Some((blob,)) => prost::Message::decode(&*blob).map_err(OfsError::ProtoDecode),
            None => {
                // Return an empty registry if none exists yet
                Ok(proto::Registry {
                    entities: Vec::new(),
                    feature_views: Vec::new(),
                    data_sources: Vec::new(),
                    on_demand_feature_views: Vec::new(),
                    stream_feature_views: Vec::new(),
                    feature_services: Vec::new(),
                    infra: None,
                    registry_schema_version: "1".to_string(),
                    version_id: String::new(),
                    last_updated: None,
                    projects: Vec::new(),
                })
            }
        }
    }

    /// Serialize and store the registry blob for a project.
    async fn save_registry(&self, project: &str, registry: &proto::Registry) -> OfsResult<()> {
        let mut buf = Vec::new();
        prost::Message::encode(registry, &mut buf).map_err(OfsError::ProtoEncode)?;

        sqlx::query(
            "INSERT INTO project_registries (project_name, serialized_registry, version, last_updated)
             VALUES (?, ?, 1, datetime('now'))
             ON CONFLICT(project_name) DO UPDATE SET
               serialized_registry = excluded.serialized_registry,
               version = version + 1,
               last_updated = datetime('now')",
        )
        .bind(project)
        .bind(&buf)
        .execute(&self.pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;

        Ok(())
    }

    /// Helper: load, modify, save.
    async fn update_registry<F>(&self, project: &str, f: F) -> OfsResult<()>
    where
        F: FnOnce(&mut proto::Registry),
    {
        let mut registry = self.load_registry(project).await?;
        f(&mut registry);
        self.save_registry(project, &registry).await
    }
}

#[async_trait]
impl Registry for SqlRegistry {
    // -----------------------------------------------------------------------
    //  Entity operations
    // -----------------------------------------------------------------------

    async fn apply_entity(&self, entity: &Entity, project: &str) -> OfsResult<()> {
        let mut e = entity.clone();
        e.project = project.to_string();
        let proto_entity = conversion::entity_to_proto(&e);
        self.update_registry(project, |r| {
            if let Some(pos) = r.entities.iter().position(|e| {
                e.spec.as_ref().map(|s| &s.name) == Some(&entity.name)
                    && e.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            }) {
                r.entities[pos] = proto_entity;
            } else {
                r.entities.push(proto_entity);
            }
        })
        .await
    }

    async fn get_entity(&self, name: &str, project: &str) -> OfsResult<Option<Entity>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .entities
            .iter()
            .find(|e| {
                e.spec.as_ref().map(|s| &s.name) == Some(&name.to_string())
                    && e.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            })
            .map(conversion::entity_from_proto))
    }

    async fn list_entities(&self, project: &str) -> OfsResult<Vec<Entity>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .entities
            .iter()
            .filter(|e| e.spec.as_ref().map(|s| &s.project) == Some(&project.to_string()))
            .map(conversion::entity_from_proto)
            .collect())
    }

    async fn delete_entity(&self, name: &str, project: &str) -> OfsResult<()> {
        self.update_registry(project, |r| {
            r.entities.retain(|e| {
                e.spec.as_ref().map(|s| &s.name) != Some(&name.to_string())
                    || e.spec.as_ref().map(|s| &s.project) != Some(&project.to_string())
            });
        })
        .await
    }

    // -----------------------------------------------------------------------
    //  FeatureView operations
    // -----------------------------------------------------------------------

    async fn apply_feature_view(&self, fv: &FeatureView, project: &str) -> OfsResult<()> {
        let mut f = fv.clone();
        f.project = project.to_string();
        let proto_fv = conversion::feature_view_to_proto(&f);
        self.update_registry(project, |r| {
            if let Some(pos) = r.feature_views.iter().position(|f| {
                f.spec.as_ref().map(|s| &s.name) == Some(&fv.name)
                    && f.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            }) {
                r.feature_views[pos] = proto_fv;
            } else {
                r.feature_views.push(proto_fv);
            }
        })
        .await
    }

    async fn get_feature_view(&self, name: &str, project: &str) -> OfsResult<Option<FeatureView>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .feature_views
            .iter()
            .find(|f| {
                f.spec.as_ref().map(|s| &s.name) == Some(&name.to_string())
                    && f.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            })
            .map(conversion::feature_view_from_proto))
    }

    async fn list_feature_views(&self, project: &str) -> OfsResult<Vec<FeatureView>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .feature_views
            .iter()
            .filter(|f| f.spec.as_ref().map(|s| &s.project) == Some(&project.to_string()))
            .map(conversion::feature_view_from_proto)
            .collect())
    }

    async fn delete_feature_view(&self, name: &str, project: &str) -> OfsResult<()> {
        self.update_registry(project, |r| {
            r.feature_views.retain(|f| {
                f.spec.as_ref().map(|s| &s.name) != Some(&name.to_string())
                    || f.spec.as_ref().map(|s| &s.project) != Some(&project.to_string())
            });
        })
        .await
    }

    // -----------------------------------------------------------------------
    //  FeatureService operations
    // -----------------------------------------------------------------------

    async fn apply_feature_service(&self, fs: &FeatureService, project: &str) -> OfsResult<()> {
        let mut f = fs.clone();
        f.project = project.to_string();
        let proto_fs = conversion::feature_service_to_proto(&f);
        self.update_registry(project, |r| {
            if let Some(pos) = r.feature_services.iter().position(|f| {
                f.spec.as_ref().map(|s| &s.name) == Some(&fs.name)
                    && f.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            }) {
                r.feature_services[pos] = proto_fs;
            } else {
                r.feature_services.push(proto_fs);
            }
        })
        .await
    }

    async fn get_feature_service(
        &self,
        name: &str,
        project: &str,
    ) -> OfsResult<Option<FeatureService>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .feature_services
            .iter()
            .find(|f| {
                f.spec.as_ref().map(|s| &s.name) == Some(&name.to_string())
                    && f.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            })
            .map(conversion::feature_service_from_proto))
    }

    async fn list_feature_services(&self, project: &str) -> OfsResult<Vec<FeatureService>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .feature_services
            .iter()
            .filter(|f| f.spec.as_ref().map(|s| &s.project) == Some(&project.to_string()))
            .map(conversion::feature_service_from_proto)
            .collect())
    }

    async fn delete_feature_service(&self, name: &str, project: &str) -> OfsResult<()> {
        self.update_registry(project, |r| {
            r.feature_services.retain(|f| {
                f.spec.as_ref().map(|s| &s.name) != Some(&name.to_string())
                    || f.spec.as_ref().map(|s| &s.project) != Some(&project.to_string())
            });
        })
        .await
    }

    // -----------------------------------------------------------------------
    //  DataSource operations
    // -----------------------------------------------------------------------

    async fn apply_data_source(&self, ds: &DataSource, project: &str) -> OfsResult<()> {
        let mut d = ds.clone();
        d.project = project.to_string();
        let proto_ds = conversion::data_source_to_proto(&d);
        self.update_registry(project, |r| {
            if let Some(pos) = r
                .data_sources
                .iter()
                .position(|d| d.name == ds.name && d.project == project)
            {
                r.data_sources[pos] = proto_ds;
            } else {
                r.data_sources.push(proto_ds);
            }
        })
        .await
    }

    async fn get_data_source(&self, name: &str, project: &str) -> OfsResult<Option<DataSource>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .data_sources
            .iter()
            .find(|d| d.name == name && d.project == project)
            .map(conversion::data_source_from_proto))
    }

    async fn list_data_sources(&self, project: &str) -> OfsResult<Vec<DataSource>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .data_sources
            .iter()
            .filter(|d| d.project == project)
            .map(conversion::data_source_from_proto)
            .collect())
    }

    async fn delete_data_source(&self, name: &str, project: &str) -> OfsResult<()> {
        self.update_registry(project, |r| {
            r.data_sources
                .retain(|d| !(d.name == name && d.project == project));
        })
        .await
    }

    // -----------------------------------------------------------------------
    //  OnDemandFeatureView operations
    // -----------------------------------------------------------------------

    async fn apply_on_demand_feature_view(
        &self,
        odfv: &OnDemandFeatureView,
        project: &str,
    ) -> OfsResult<()> {
        let mut o = odfv.clone();
        o.project = project.to_string();
        let proto_odfv = conversion::odfv_to_proto(&o);
        self.update_registry(project, |r| {
            if let Some(pos) = r.on_demand_feature_views.iter().position(|o| {
                o.spec.as_ref().map(|s| &s.name) == Some(&odfv.name)
                    && o.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            }) {
                r.on_demand_feature_views[pos] = proto_odfv;
            } else {
                r.on_demand_feature_views.push(proto_odfv);
            }
        })
        .await
    }

    async fn list_on_demand_feature_views(
        &self,
        project: &str,
    ) -> OfsResult<Vec<OnDemandFeatureView>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .on_demand_feature_views
            .iter()
            .filter(|o| o.spec.as_ref().map(|s| &s.project) == Some(&project.to_string()))
            .map(conversion::odfv_from_proto)
            .collect())
    }

    // -----------------------------------------------------------------------
    //  Materialization operations
    // -----------------------------------------------------------------------

    async fn apply_materialization(
        &self,
        fv_name: &str,
        project: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> OfsResult<()> {
        self.update_registry(project, |r| {
            if let Some(fv) = r.feature_views.iter_mut().find(|f| {
                f.spec.as_ref().map(|s| &s.name) == Some(&fv_name.to_string())
                    && f.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            }) {
                let meta = fv.meta.get_or_insert_with(|| proto::FeatureViewMeta {
                    created_timestamp: None,
                    last_updated_timestamp: None,
                    materialization_intervals: Vec::new(),
                    current_version_number: 1,
                    version_id: String::new(),
                    state: 0,
                });
                let new_start = prost_types::Timestamp {
                    seconds: start.timestamp(),
                    nanos: start.timestamp_subsec_nanos() as i32,
                };
                let new_end = prost_types::Timestamp {
                    seconds: end.timestamp(),
                    nanos: end.timestamp_subsec_nanos() as i32,
                };
                let exists = meta.materialization_intervals.iter().any(|i| {
                    i.start_time.as_ref() == Some(&new_start)
                        && i.end_time.as_ref() == Some(&new_end)
                });
                if !exists {
                    meta.materialization_intervals
                        .push(proto::MaterializationInterval {
                            start_time: Some(new_start),
                            end_time: Some(new_end),
                        });
                }
            }
        })
        .await
    }

    async fn get_materialization_intervals(
        &self,
        fv_name: &str,
        project: &str,
    ) -> OfsResult<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
        let registry = self.load_registry(project).await?;
        Ok(registry
            .feature_views
            .iter()
            .find(|f| {
                f.spec.as_ref().map(|s| &s.name) == Some(&fv_name.to_string())
                    && f.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            })
            .and_then(|f| f.meta.as_ref())
            .map(|m| conversion::intervals_from_proto(&m.materialization_intervals))
            .unwrap_or_default())
    }

    async fn remove_materialization_intervals(
        &self,
        fv_name: &str,
        project: &str,
        intervals: &[(DateTime<Utc>, DateTime<Utc>)],
    ) -> OfsResult<()> {
        self.update_registry(project, |r| {
            if let Some(fv) = r.feature_views.iter_mut().find(|f| {
                f.spec.as_ref().map(|s| &s.name) == Some(&fv_name.to_string())
                    && f.spec.as_ref().map(|s| &s.project) == Some(&project.to_string())
            }) {
                let meta = fv.meta.get_or_insert_with(|| proto::FeatureViewMeta {
                    created_timestamp: None,
                    last_updated_timestamp: None,
                    materialization_intervals: Vec::new(),
                    current_version_number: 1,
                    version_id: String::new(),
                    state: 0,
                });
                let remove_set: Vec<(i64, i32)> = intervals
                    .iter()
                    .map(|(s, _e)| (s.timestamp(), s.timestamp_subsec_nanos() as i32))
                    .collect();
                meta.materialization_intervals.retain(|i| {
                    let start_secs = i.start_time.as_ref().map(|t| t.seconds).unwrap_or(0);
                    let start_nanos = i.start_time.as_ref().map(|t| t.nanos).unwrap_or(0);
                    !remove_set.contains(&(start_secs, start_nanos))
                });
            }
        })
        .await
    }

    async fn commit(&self) -> OfsResult<()> {
        // SQL writes are auto-committed; nothing extra to do.
        Ok(())
    }

    async fn create_backfill_job(&self, job: &BackfillJob) -> OfsResult<()> {
        let status_str = match job.status {
            BackfillStatus::Pending => "Pending",
            BackfillStatus::Running => "Running",
            BackfillStatus::Completed => "Completed",
            BackfillStatus::Failed => "Failed",
            BackfillStatus::Cancelled => "Cancelled",
        };
        sqlx::query(
            "INSERT INTO backfill_jobs (id, feature_view_name, project, start_ts, end_ts, status, progress, chunk_size_seconds, error, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(&job.id)
        .bind(&job.feature_view_name)
        .bind(&job.project)
        .bind(job.start.to_rfc3339())
        .bind(job.end.to_rfc3339())
        .bind(status_str)
        .bind(job.progress)
        .bind(job.chunk_size_seconds)
        .bind(&job.error)
        .bind(job.created_at.to_rfc3339())
        .bind(job.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_backfill_job(&self, job_id: &str) -> OfsResult<Option<BackfillJob>> {
        let row: Option<(String, String, String, String, String, String, f64, i64, Option<String>, String, String)> =
            sqlx::query_as(
                "SELECT id, feature_view_name, project, start_ts, end_ts, status, progress, chunk_size_seconds, error, created_at, updated_at FROM backfill_jobs WHERE id = ?",
            )
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;

        match row {
            Some((
                id,
                fv_name,
                project,
                start_str,
                end_str,
                status_str,
                progress,
                chunk_secs,
                error,
                created_str,
                updated_str,
            )) => {
                let status = match status_str.as_str() {
                    "Running" => BackfillStatus::Running,
                    "Completed" => BackfillStatus::Completed,
                    "Failed" => BackfillStatus::Failed,
                    "Cancelled" => BackfillStatus::Cancelled,
                    _ => BackfillStatus::Pending,
                };
                let start = DateTime::parse_from_rfc3339(&start_str)
                    .map(|d| d.with_timezone(&Utc))
                    .map_err(|e| OfsError::Database(e.to_string()))?;
                let end = DateTime::parse_from_rfc3339(&end_str)
                    .map(|d| d.with_timezone(&Utc))
                    .map_err(|e| OfsError::Database(e.to_string()))?;
                let created = DateTime::parse_from_rfc3339(&created_str)
                    .map(|d| d.with_timezone(&Utc))
                    .map_err(|e| OfsError::Database(e.to_string()))?;
                let updated = DateTime::parse_from_rfc3339(&updated_str)
                    .map(|d| d.with_timezone(&Utc))
                    .map_err(|e| OfsError::Database(e.to_string()))?;
                Ok(Some(BackfillJob {
                    id,
                    feature_view_name: fv_name,
                    project,
                    start,
                    end,
                    status,
                    progress,
                    chunk_size_seconds: chunk_secs,
                    error,
                    created_at: created,
                    updated_at: updated,
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_backfill_jobs(&self, project: &str) -> OfsResult<Vec<BackfillJob>> {
        let rows: Vec<(String, String, String, String, String, String, f64, i64, Option<String>, String, String)> =
            sqlx::query_as(
                "SELECT id, feature_view_name, project, start_ts, end_ts, status, progress, chunk_size_seconds, error, created_at, updated_at FROM backfill_jobs WHERE project = ? ORDER BY created_at DESC",
            )
            .bind(project)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;

        let mut jobs = Vec::with_capacity(rows.len());
        for (
            id,
            fv_name,
            project,
            start_str,
            end_str,
            status_str,
            progress,
            chunk_secs,
            error,
            created_str,
            updated_str,
        ) in rows
        {
            let status = match status_str.as_str() {
                "Running" => BackfillStatus::Running,
                "Completed" => BackfillStatus::Completed,
                "Failed" => BackfillStatus::Failed,
                "Cancelled" => BackfillStatus::Cancelled,
                _ => BackfillStatus::Pending,
            };
            let start = DateTime::parse_from_rfc3339(&start_str)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| OfsError::Database(e.to_string()))?;
            let end = DateTime::parse_from_rfc3339(&end_str)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| OfsError::Database(e.to_string()))?;
            let created = DateTime::parse_from_rfc3339(&created_str)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| OfsError::Database(e.to_string()))?;
            let updated = DateTime::parse_from_rfc3339(&updated_str)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| OfsError::Database(e.to_string()))?;
            jobs.push(BackfillJob {
                id,
                feature_view_name: fv_name,
                project,
                start,
                end,
                status,
                progress,
                chunk_size_seconds: chunk_secs,
                error,
                created_at: created,
                updated_at: updated,
            });
        }
        Ok(jobs)
    }

    async fn update_backfill_job(&self, job: &BackfillJob) -> OfsResult<()> {
        let status_str = match job.status {
            BackfillStatus::Pending => "Pending",
            BackfillStatus::Running => "Running",
            BackfillStatus::Completed => "Completed",
            BackfillStatus::Failed => "Failed",
            BackfillStatus::Cancelled => "Cancelled",
        };
        sqlx::query(
            "UPDATE backfill_jobs SET status = ?1, progress = ?2, error = ?3, updated_at = ?4 WHERE id = ?5",
        )
        .bind(status_str)
        .bind(job.progress)
        .bind(&job.error)
        .bind(job.updated_at.to_rfc3339())
        .bind(&job.id)
        .execute(&self.pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ofs_core::types::{DataSourceOptions, Feature, FileFormat};
    use ofs_core::value_type::ValueType;

    async fn create_test_registry() -> SqlRegistry {
        SqlRegistry::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_entity_crud() {
        let reg = create_test_registry().await;

        let entity = Entity::new("driver", vec!["driver_id".to_string()]);
        reg.apply_entity(&entity, "default").await.unwrap();

        let got = reg.get_entity("driver", "default").await.unwrap().unwrap();
        assert_eq!(got.name, "driver");
        assert_eq!(got.join_keys, vec!["driver_id"]);

        let entities = reg.list_entities("default").await.unwrap();
        assert_eq!(entities.len(), 1);

        reg.delete_entity("driver", "default").await.unwrap();
        let got = reg.get_entity("driver", "default").await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn test_feature_view_crud() {
        let reg = create_test_registry().await;

        let mut fv = FeatureView::new("driver_stats");
        fv.features
            .push(Feature::new("conv_rate", ValueType::Double));
        fv.features
            .push(Feature::new("acc_rate", ValueType::Double));
        fv.entities.push("driver".to_string());
        fv.online = true;

        reg.apply_feature_view(&fv, "default").await.unwrap();

        let got = reg
            .get_feature_view("driver_stats", "default")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.name, "driver_stats");
        assert_eq!(got.features.len(), 2);
        assert!(got.online);

        let list = reg.list_feature_views("default").await.unwrap();
        assert_eq!(list.len(), 1);

        reg.delete_feature_view("driver_stats", "default")
            .await
            .unwrap();
        let got = reg
            .get_feature_view("driver_stats", "default")
            .await
            .unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn test_feature_service_crud() {
        let reg = create_test_registry().await;

        let fs = FeatureService::new("model_v1");
        reg.apply_feature_service(&fs, "default").await.unwrap();

        let got = reg
            .get_feature_service("model_v1", "default")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.name, "model_v1");

        reg.delete_feature_service("model_v1", "default")
            .await
            .unwrap();
        let got = reg
            .get_feature_service("model_v1", "default")
            .await
            .unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn test_data_source_crud() {
        let reg = create_test_registry().await;

        let ds = DataSource::new(
            "my_features",
            DataSourceOptions::File {
                path: "data/features.parquet".to_string(),
                file_format: FileFormat::Parquet,
                s3_endpoint_override: None,
            },
        );
        reg.apply_data_source(&ds, "default").await.unwrap();

        let got = reg
            .get_data_source("my_features", "default")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.name, "my_features");

        reg.delete_data_source("my_features", "default")
            .await
            .unwrap();
        let got = reg.get_data_source("my_features", "default").await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn test_materialization_intervals() {
        let reg = create_test_registry().await;

        let fv = FeatureView::new("driver_stats");
        reg.apply_feature_view(&fv, "default").await.unwrap();

        let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        reg.apply_materialization("driver_stats", "default", start, end)
            .await
            .unwrap();

        let intervals = reg
            .get_materialization_intervals("driver_stats", "default")
            .await
            .unwrap();
        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].0, start);
        assert_eq!(intervals[0].1, end);
    }

    #[tokio::test]
    async fn test_project_isolation() {
        let reg = create_test_registry().await;

        let e1 = Entity::new("driver", vec!["driver_id".to_string()]);
        let e2 = Entity::new("customer", vec!["customer_id".to_string()]);

        reg.apply_entity(&e1, "proj_a").await.unwrap();
        reg.apply_entity(&e2, "proj_b").await.unwrap();

        let list_a = reg.list_entities("proj_a").await.unwrap();
        assert_eq!(list_a.len(), 1);
        assert_eq!(list_a[0].name, "driver");

        let list_b = reg.list_entities("proj_b").await.unwrap();
        assert_eq!(list_b.len(), 1);
        assert_eq!(list_b[0].name, "customer");

        // Entity from proj_a should not be found in proj_b
        let got = reg.get_entity("driver", "proj_b").await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn test_reapply_updates() {
        let reg = create_test_registry().await;

        let mut entity = Entity::new("driver", vec!["driver_id".to_string()]);
        entity.description = "original".to_string();
        reg.apply_entity(&entity, "default").await.unwrap();

        entity.description = "updated".to_string();
        reg.apply_entity(&entity, "default").await.unwrap();

        let got = reg.get_entity("driver", "default").await.unwrap().unwrap();
        assert_eq!(got.description, "updated");
    }

    #[tokio::test]
    async fn test_commit_noop() {
        let reg = create_test_registry().await;
        // commit() should not error on empty registry
        reg.commit().await.unwrap();

        // commit() should not error after operations
        let entity = Entity::new("driver", vec!["driver_id".to_string()]);
        reg.apply_entity(&entity, "default").await.unwrap();
        reg.commit().await.unwrap();
    }
}
