use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ofs_core::entity_key::serialize_entity_key_v3;
use ofs_core::errors::{OfsError, OfsResult};
use ofs_core::traits::{
    FeatureVector, FieldStatus, OnlineReadResponse, OnlineResponseMetadata, OnlineStore,
};
use ofs_core::types::{EntityKey, FeatureViewWithProjection, OnlineWriteRecord};
use sqlx::{Row, SqlitePool};

/// SQLite-based online store.
///
/// Each feature view gets its own table: `{project}__{fv_name}`
/// Tables have columns: `entity_key TEXT PRIMARY KEY`, one BLOB column per feature, `event_ts TEXT`.
pub struct SqliteOnlineStore {
    pool: SqlitePool,
}

impl SqliteOnlineStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn in_memory() -> OfsResult<Self> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(Self::new(pool))
    }

    fn table_name(project: &str, fv_name: &str) -> String {
        format!("__ofs_{}__{}", project, fv_name)
    }

    async fn ensure_table(&self, project: &str, fv: &FeatureViewWithProjection) -> OfsResult<()> {
        let table = Self::table_name(project, &fv.feature_view.name);
        let mut col_defs = vec!["entity_key TEXT PRIMARY KEY NOT NULL".to_string()];

        for feature in &fv.projection.feature_columns {
            col_defs.push(format!("\"{}\" BLOB", feature.name));
        }
        col_defs.push("event_ts TEXT NOT NULL".to_string());

        let ddl = format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" ({})",
            table,
            col_defs.join(", ")
        );

        sqlx::query(&ddl)
            .execute(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl OnlineStore for SqliteOnlineStore {
    async fn online_read(
        &self,
        entity_keys: Vec<EntityKey>,
        features: &[FeatureViewWithProjection],
        project: &str,
    ) -> OfsResult<OnlineReadResponse> {
        let mut all_feature_names = Vec::new();
        let mut all_results = Vec::new();

        for fvp in features {
            let table = Self::table_name(project, &fvp.feature_view.name);
            self.ensure_table(project, fvp).await?;

            let feature_names: Vec<String> = fvp
                .projection
                .feature_columns
                .iter()
                .map(|f| f.name.clone())
                .collect();

            let quoted_cols: Vec<String> =
                feature_names.iter().map(|n| format!("\"{}\"", n)).collect();

            let select_cols = if quoted_cols.is_empty() {
                "entity_key, event_ts".to_string()
            } else {
                format!("entity_key, {}, event_ts", quoted_cols.join(", "))
            };

            for ek in &entity_keys {
                let serialized = hex::encode(serialize_entity_key_v3(ek));

                // Use query + manual row mapping instead of query_as with complex tuple
                let sql = format!(
                    "SELECT {} FROM \"{}\" WHERE entity_key = ?",
                    select_cols, table
                );

                let row_opt = sqlx::query(&sql)
                    .bind(&serialized)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| OfsError::Database(e.to_string()))?;

                if let Some(row) = row_opt {
                    let ts_str: String = row.get("event_ts");
                    let ts = DateTime::parse_from_rfc3339(&ts_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok();

                    let mut vals = Vec::new();
                    let mut statuses = Vec::new();
                    let mut event_timestamps = Vec::new();

                    for fname in &feature_names {
                        let col_name = format!("{}__{}", fvp.feature_view.name, fname);
                        all_feature_names.push(col_name);

                        let val: Option<Vec<u8>> = row.get(fname.as_str());
                        match val {
                            Some(v) => {
                                vals.push(v);
                                statuses.push(FieldStatus::Present);
                            }
                            None => {
                                vals.push(Vec::new());
                                statuses.push(FieldStatus::NullValue);
                            }
                        }
                        event_timestamps.push(ts);
                    }

                    all_results.push(FeatureVector {
                        values: vals,
                        statuses,
                        event_timestamps,
                    });
                } else {
                    let mut statuses = Vec::new();
                    let mut event_timestamps = Vec::new();
                    for fname in &feature_names {
                        let col_name = format!("{}__{}", fvp.feature_view.name, fname);
                        all_feature_names.push(col_name);
                        statuses.push(FieldStatus::NotFound);
                        event_timestamps.push(None);
                    }
                    all_results.push(FeatureVector {
                        values: vec![Vec::new(); feature_names.len()],
                        statuses,
                        event_timestamps,
                    });
                }
            }
        }

        Ok(OnlineReadResponse {
            metadata: OnlineResponseMetadata {
                feature_names: all_feature_names,
            },
            results: all_results,
        })
    }

    async fn online_write_batch(
        &self,
        data: Vec<OnlineWriteRecord>,
        project: &str,
    ) -> OfsResult<()> {
        for record in &data {
            let table = Self::table_name(project, &record.feature_view_name);
            let serialized_key = hex::encode(serialize_entity_key_v3(&record.entity_key));
            let ts_str = record
                .timestamp
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string();

            if record.values.is_empty() {
                continue;
            }

            // Auto-create table with columns from the values
            let mut col_defs = vec!["entity_key TEXT PRIMARY KEY NOT NULL".to_string()];
            for key in record.values.keys() {
                col_defs.push(format!("\"{}\" BLOB", key));
            }
            col_defs.push("event_ts TEXT NOT NULL".to_string());
            let ddl = format!(
                "CREATE TABLE IF NOT EXISTS \"{}\" ({})",
                table,
                col_defs.join(", ")
            );
            sqlx::query(&ddl)
                .execute(&self.pool)
                .await
                .map_err(|e| OfsError::Database(e.to_string()))?;

            // Build dynamic INSERT OR REPLACE
            let col_names: Vec<String> =
                record.values.keys().map(|k| format!("\"{}\"", k)).collect();
            let placeholders: Vec<String> = (0..record.values.len())
                .map(|i| format!("?{}", i + 2))
                .collect();

            let sql = format!(
                "INSERT OR REPLACE INTO \"{}\" (entity_key, {}, event_ts) VALUES (?1, {}, ?{})",
                table,
                col_names.join(", "),
                placeholders.join(", "),
                record.values.len() + 2
            );

            let mut q = sqlx::query(&sql).bind(&serialized_key);
            for val in record.values.values() {
                q = q.bind(val);
            }
            q = q.bind(&ts_str);

            q.execute(&self.pool)
                .await
                .map_err(|e| OfsError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn update(
        &self,
        tables_to_keep: Vec<String>,
        tables_to_delete: Vec<String>,
    ) -> OfsResult<()> {
        for table in &tables_to_delete {
            let sql = format!("DROP TABLE IF EXISTS \"{}\"", table);
            sqlx::query(&sql)
                .execute(&self.pool)
                .await
                .map_err(|e| OfsError::Database(e.to_string()))?;
        }
        let _ = tables_to_keep;
        Ok(())
    }

    async fn purge_expired(
        &self,
        feature_view_name: &str,
        project: &str,
        cutoff: DateTime<Utc>,
    ) -> OfsResult<u64> {
        let table = Self::table_name(project, feature_view_name);
        let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let sql = format!("DELETE FROM \"{}\" WHERE event_ts < ?", table);
        let result = sqlx::query(&sql)
            .bind(&cutoff_str)
            .execute(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(result.rows_affected())
    }

    async fn teardown(&self) -> OfsResult<()> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '__ofs_%'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;

        for (name,) in rows {
            let sql = format!("DROP TABLE IF EXISTS \"{}\"", name);
            sqlx::query(&sql)
                .execute(&self.pool)
                .await
                .map_err(|e| OfsError::Database(e.to_string()))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ofs_core::types::{EntityKey, Feature, FeatureView, FeatureViewProjection};
    use ofs_core::value_type::ValueType;
    use std::collections::HashMap;

    async fn create_store() -> SqliteOnlineStore {
        SqliteOnlineStore::in_memory().await.unwrap()
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
                feature_columns: fv_features.clone(),
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
    async fn test_write_then_read() {
        let store = create_store().await;
        let now = Utc::now();

        let ek = EntityKey::new(vec!["driver_id".to_string()]);
        let mut values = HashMap::new();
        values.insert("conv_rate".to_string(), b"0.85".to_vec());
        values.insert("acc_rate".to_string(), b"0.92".to_vec());

        let record = OnlineWriteRecord {
            entity_key: ek.clone(),
            values,
            timestamp: now,
            feature_view_name: "driver_stats".to_string(),
        };

        store
            .online_write_batch(vec![record], "default")
            .await
            .unwrap();

        let fvp = make_fvp("driver_stats", vec!["conv_rate", "acc_rate"]);
        let response = store
            .online_read(vec![ek], &[fvp], "default")
            .await
            .unwrap();

        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].values.len(), 2);
        assert_eq!(response.results[0].values[0], b"0.85");
        assert_eq!(response.results[0].values[1], b"0.92");
    }

    #[tokio::test]
    async fn test_read_missing_entity() {
        let store = create_store().await;

        let ek = EntityKey::new(vec!["missing_id".to_string()]);
        let fvp = make_fvp("driver_stats", vec!["conv_rate"]);

        let response = store
            .online_read(vec![ek], &[fvp], "default")
            .await
            .unwrap();

        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].statuses[0], FieldStatus::NotFound);
    }

    #[tokio::test]
    async fn test_teardown() {
        let store = create_store().await;
        let now = Utc::now();

        let ek = EntityKey::new(vec!["driver_id".to_string()]);
        let mut values = HashMap::new();
        values.insert("conv_rate".to_string(), b"0.85".to_vec());

        let record = OnlineWriteRecord {
            entity_key: ek,
            values,
            timestamp: now,
            feature_view_name: "driver_stats".to_string(),
        };
        store
            .online_write_batch(vec![record], "default")
            .await
            .unwrap();

        store.teardown().await.unwrap();

        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name LIKE '__ofs_%'",
        )
        .fetch_one(&store.pool)
        .await
        .unwrap();
        assert_eq!(table_count, 0);
    }
}
