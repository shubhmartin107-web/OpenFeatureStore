# ofs-offline-store

DuckDB-powered offline store for point-in-time correct feature retrieval.

## DuckDbOfflineStore

```rust
pub struct DuckDbOfflineStore;

impl OfflineStore for DuckDbOfflineStore {
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
}
```

## ASOF JOIN

The offline store builds SQL queries using DuckDB's `ASOF JOIN` capability for
point-in-time correctness:

```sql
SELECT
    entity_df.entity_key,
    entity_df.event_timestamp,
    fv.feature_name,
    fv.feature_value
FROM entity_dataframe entity_df
ASOF JOIN feature_table fv
    ON entity_df.entity_key = fv.entity_key
    AND entity_df.event_timestamp >= fv.event_timestamp
WHERE
    entity_df.event_timestamp >= '2024-01-01'
    AND entity_df.event_timestamp < '2024-01-02'
```

## RetrievalJob

The offline store returns a `RetrievalJob` with the generated SQL query and
column metadata. The actual DuckDB execution is deferred to the caller
(typically the materialization engine):

```rust
pub struct RetrievalJob {
    pub query: String,
    pub schema_fields: Vec<String>,
}
```
