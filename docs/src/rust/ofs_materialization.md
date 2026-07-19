# ofs-materialization

The materialization engine bridges the offline and online stores,
pulling feature data from DuckDB and writing it to SQLite or Redis.

## DefaultMaterializationEngine

```rust
pub struct DefaultMaterializationEngine;

impl DefaultMaterializationEngine {
    pub fn new(
        registry: Arc<dyn Registry>,
        offline_store: Arc<dyn OfflineStore>,
        online_store: Arc<dyn OnlineStore>,
        config: RepoConfig,
    ) -> Self;
}
```

## Methods

```rust
#[async_trait]
impl MaterializationEngine for DefaultMaterializationEngine {
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
```

## Execution Flow

1. Registry returns all feature views (or specified subset)
2. For each feature view:
   - Call `offline_store.pull_features(fv, start, end)` → SQL query
   - Execute DuckDB query directly
   - Convert each row to `OnlineWriteRecord`
   - Write records to online store in batches of 100

## Incremental Materialization

`materialize_incremental` uses previously recorded materialization intervals to
determine the start time:

- If no prior materialization exists, materializes from epoch to `end_date`
- If prior intervals exist, materializes from `last_end_time` to `end_date`
- New intervals are recorded in the registry
