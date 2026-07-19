use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ofs_core::errors::OfsResult;
use ofs_core::traits::{EntityDataFrame, OfflineStore, RetrievalJob};
use ofs_core::types::{FeatureView, FeatureViewWithProjection, RepoConfig};

/// DuckDB-based offline store.
///
/// Builds ASOF LEFT JOIN queries for point-in-time correct feature retrieval.
/// The actual query execution is deferred; `RetrievalJob` contains the SQL string
/// that the caller runs against DuckDB.
pub struct DuckDbOfflineStore;

impl DuckDbOfflineStore {
    /// Build an ASOF LEFT JOIN query for point-in-time correct feature retrieval.
    fn build_asof_query(
        entity_table: &str,
        entity_ts_col: &str,
        entity_key_cols: &[String],
        features: &[FeatureViewWithProjection],
    ) -> String {
        let mut selects = vec![format!("\"{}\".*", entity_table)];
        let mut joins = Vec::new();

        let key_condition = if entity_key_cols.len() == 1 {
            format!(
                "\"{e}\".\"{k}\" = \"fv{{i}}\".\"{k}\"",
                e = entity_table,
                k = entity_key_cols[0],
            )
        } else {
            let parts: Vec<String> = entity_key_cols
                .iter()
                .map(|k| {
                    format!(
                        "\"{e}\".\"{k}\" = \"fv{{i}}\".\"{k}\"",
                        e = entity_table,
                        k = k
                    )
                })
                .collect();
            parts.join(" AND ")
        };

        for (i, fvp) in features.iter().enumerate() {
            let fv_name = &fvp.feature_view.name;
            let table_name = format!("fv_{}", i);

            for feature in &fvp.feature_view.features {
                let col_name = if let Some(alias) = &fvp.projection.feature_view_name_alias {
                    format!("\"{alias}__{feat}\"", alias = alias, feat = feature.name)
                } else {
                    format!(
                        "\"{fv_name}__{feat}\"",
                        fv_name = fv_name,
                        feat = feature.name
                    )
                };

                selects.push(format!(
                    "\"{table}\".\"{feat}\" AS {col}",
                    table = table_name,
                    feat = feature.name,
                    col = col_name
                ));
            }

            fn sanitize_path(p: &str) -> String {
                p.replace('\'', "''")
            }

            let batch_source_path = fvp
                .feature_view
                .batch_source
                .as_ref()
                .and_then(|ds| match &ds.options {
                    ofs_core::types::DataSourceOptions::File { path, .. } => {
                        Some(sanitize_path(path))
                    }
                    _ => None,
                });

            let data_source = match batch_source_path {
                Some(p) if p.ends_with(".parquet") => {
                    format!("read_parquet('{}')", p)
                }
                Some(p) if p.ends_with(".csv") => {
                    format!("read_csv_auto('{}')", p)
                }
                Some(p) => {
                    format!("read_parquet('{}')", p)
                }
                None => {
                    // Placeholder when no batch source is available
                    format!(
                        "(SELECT * FROM (VALUES(NULL::VARCHAR)) AS t({key}) WHERE 1=0)",
                        key = entity_key_cols
                            .first()
                            .map(|k| format!("\"{k}\" VARCHAR"))
                            .unwrap_or_else(|| "\"key\" VARCHAR".to_string())
                    )
                }
            };

            let current_key_cond = key_condition.replace("{i}", &i.to_string());
            joins.push(format!(
                "ASOF LEFT JOIN {data} AS \"{table}\"
                 ON {key_cond}
                 AND \"{entity}\".\"{ts}\" >= \"{table}\".\"event_timestamp\"",
                data = data_source,
                table = table_name,
                key_cond = current_key_cond,
                entity = entity_table,
                ts = entity_ts_col,
            ));
        }

        format!(
            "SELECT {} FROM \"{}\" {}",
            selects.join(",\n"),
            entity_table,
            joins.join("\n")
        )
    }

    /// Build the full query: entity CTE (or view reference) + ASOF joins.
    fn build_full_query(
        entity_df: &EntityDataFrame,
        features: &[FeatureViewWithProjection],
    ) -> String {
        let asof_query = Self::build_asof_query(
            "entity_df",
            &entity_df.timestamp_column,
            &entity_df.entity_key_columns,
            features,
        );

        format!(
            "-- Entity data should be registered as a view named \"entity_df\"\n\
             -- with columns: {}\n\
             -- timestamp_column: {}, entity_key_columns: {:?}\n\n\
             WITH \"entity_df\" AS (\n\
             -- Replace this CTE with your actual entity dataframe\n\
             -- Register via: conn.execute_batch(\"CREATE VIEW entity_df AS SELECT ...\")\n\
             SELECT * FROM (VALUES(NULL::VARCHAR)) AS t(key) WHERE 1=0\n\
             )\n{}\n",
            entity_df.columns.join(", "),
            entity_df.timestamp_column,
            entity_df.entity_key_columns,
            asof_query
        )
    }
}

#[async_trait]
impl OfflineStore for DuckDbOfflineStore {
    async fn get_historical_features(
        &self,
        entity_df: EntityDataFrame,
        features: Vec<FeatureViewWithProjection>,
        _config: &RepoConfig,
    ) -> OfsResult<RetrievalJob> {
        let query = Self::build_full_query(&entity_df, &features);

        // Derive output schema
        let mut schema_fields = entity_df.columns.clone();
        for fvp in &features {
            for feature in &fvp.feature_view.features {
                let col_name = if let Some(alias) = &fvp.projection.feature_view_name_alias {
                    format!("{}__{}", alias, feature.name)
                } else {
                    format!("{}__{}", fvp.feature_view.name, feature.name)
                };
                schema_fields.push(col_name);
            }
        }

        Ok(RetrievalJob {
            query,
            schema_fields,
        })
    }

    async fn pull_features(
        &self,
        feature_view: &FeatureView,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> OfsResult<RetrievalJob> {
        let batch_source_raw = feature_view
            .batch_source
            .as_ref()
            .and_then(|ds| match &ds.options {
                ofs_core::types::DataSourceOptions::File { path, .. } => Some(path.clone()),
                _ => None,
            })
            .unwrap_or_default();
        // Basic SQL injection guard for file paths embedded in SQL literals
        fn sanitize_path(p: &str) -> String {
            p.replace('\'', "''")
        }
        let batch_source = sanitize_path(&batch_source_raw);

        let start_str = start_date.format("%Y-%m-%d %H:%M:%S").to_string();
        let end_str = end_date.format("%Y-%m-%d %H:%M:%S").to_string();

        let mut schema_fields = vec!["event_timestamp".to_string()];
        for f in &feature_view.features {
            schema_fields.push(f.name.clone());
        }

        let query = if batch_source.ends_with(".parquet") {
            format!(
                "SELECT * FROM read_parquet('{path}') WHERE event_timestamp >= '{start}' AND event_timestamp < '{end}'",
                path = batch_source,
                start = start_str,
                end = end_str,
            )
        } else if batch_source.ends_with(".csv") {
            format!(
                "SELECT * FROM read_csv_auto('{path}') WHERE event_timestamp >= '{start}' AND event_timestamp < '{end}'",
                path = batch_source,
                start = start_str,
                end = end_str,
            )
        } else {
            format!(
                "SELECT * FROM '{path}' WHERE event_timestamp >= '{start}' AND event_timestamp < '{end}'",
                path = batch_source,
                start = start_str,
                end = end_str,
            )
        };

        Ok(RetrievalJob {
            query,
            schema_fields,
        })
    }

    async fn purge_offline_data(
        &self,
        _feature_view: &FeatureView,
        _project: &str,
        _cutoff: DateTime<Utc>,
    ) -> OfsResult<u64> {
        // DuckDbOfflineStore is a query builder without a database connection.
        // Offline data cleanup must be performed by the materialization engine
        // or directly against the DuckDB database.
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ofs_core::types::{
        DataSource, DataSourceOptions, Feature, FeatureView, FeatureViewProjection, FileFormat,
    };
    use ofs_core::value_type::ValueType;
    use std::collections::HashMap;

    fn make_fvp(
        name: &str,
        features: Vec<&str>,
        source_path: Option<&str>,
    ) -> FeatureViewWithProjection {
        let fv_features: Vec<Feature> = features
            .iter()
            .map(|f| Feature::new(f, ValueType::Double))
            .collect();
        let mut fv = FeatureView::new(name);
        fv.features = fv_features.clone();
        if let Some(path) = source_path {
            fv.batch_source = Some(DataSource::new(
                "source",
                DataSourceOptions::File {
                    path: path.to_string(),
                    file_format: FileFormat::Parquet,
                    s3_endpoint_override: None,
                },
            ));
        }
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

    #[test]
    fn test_build_asof_query_single_key_single_fv() {
        let fvp = make_fvp(
            "driver_stats",
            vec!["conv_rate"],
            Some("data/driver_stats.parquet"),
        );
        let query = DuckDbOfflineStore::build_asof_query(
            "entity_df",
            "event_timestamp",
            &["driver_id".to_string()],
            &[fvp],
        );

        assert!(query.contains("ASOF LEFT JOIN"));
        assert!(query.contains("driver_stats__conv_rate"));
        assert!(query.contains("event_timestamp"));
        assert!(query.contains("read_parquet"));
    }

    #[test]
    fn test_build_asof_query_multi_key() {
        let fvp = make_fvp(
            "customer_stats",
            vec!["total_spend"],
            Some("data/cust.parquet"),
        );
        let query = DuckDbOfflineStore::build_asof_query(
            "entity_df",
            "event_timestamp",
            &["customer_id".to_string(), "region".to_string()],
            &[fvp],
        );

        assert!(query.contains("ASOF LEFT JOIN"));
        assert!(query.contains("AND"));
        assert!(query.contains("customer_id"));
        assert!(query.contains("region"));
    }

    #[test]
    fn test_build_asof_query_multiple_fvs() {
        let fv1 = make_fvp(
            "driver_stats",
            vec!["conv_rate"],
            Some("data/drivers.parquet"),
        );
        let fv2 = make_fvp(
            "customer_stats",
            vec!["total_spend"],
            Some("data/cust.parquet"),
        );
        let query = DuckDbOfflineStore::build_asof_query(
            "entity_df",
            "event_timestamp",
            &["driver_id".to_string()],
            &[fv1, fv2],
        );

        assert_eq!(query.matches("ASOF LEFT JOIN").count(), 2);
        assert!(query.contains("driver_stats__conv_rate"));
        assert!(query.contains("customer_stats__total_spend"));
    }

    #[test]
    fn test_build_asof_query_no_batch_source() {
        let fvp = make_fvp("driver_stats", vec!["conv_rate"], None);
        let query = DuckDbOfflineStore::build_asof_query(
            "entity_df",
            "event_timestamp",
            &["driver_id".to_string()],
            &[fvp],
        );

        assert!(query.contains("ASOF LEFT JOIN"));
        // Should generate a dummy subquery when no source available
        assert!(query.contains("VALUES(NULL::VARCHAR)"));
    }

    #[tokio::test]
    async fn test_full_query() {
        let entity_df = EntityDataFrame {
            columns: vec!["driver_id".to_string(), "event_timestamp".to_string()],
            arrow_data: Vec::new(),
            num_rows: 0,
            timestamp_column: "event_timestamp".to_string(),
            entity_key_columns: vec!["driver_id".to_string()],
        };

        let fvp = make_fvp(
            "driver_stats",
            vec!["conv_rate"],
            Some("data/drivers.parquet"),
        );
        let job = DuckDbOfflineStore
            .get_historical_features(entity_df, vec![fvp], &RepoConfig::default())
            .await
            .unwrap();

        assert!(job.query.contains("entity_df"));
        assert!(job.query.contains("driver_stats__conv_rate"));
        assert_eq!(job.schema_fields.len(), 3);
        assert!(
            job.schema_fields
                .contains(&"driver_stats__conv_rate".to_string())
        );
    }

    #[tokio::test]
    async fn test_pull_features() {
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

        let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2024-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let job = DuckDbOfflineStore
            .pull_features(&fv, start, end)
            .await
            .unwrap();

        assert!(job.query.contains("read_parquet"));
        assert!(job.query.contains("2024-01-01"));
        assert!(job.query.contains("2024-01-02"));
        assert_eq!(job.schema_fields, vec!["event_timestamp", "conv_rate"]);
    }
}
