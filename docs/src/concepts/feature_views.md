# Feature Views

A **FeatureView** defines a group of features that are computed together from a data source.
It specifies how features are retrieved and materialized.

## Definition

```rust
pub struct FeatureView {
    pub name: String,
    pub project: String,
    pub entities: Vec<String>,        // Referenced entity names
    pub features: Vec<Feature>,       // Feature definitions
    pub tags: HashMap<String, String>,
    pub ttl: Option<Duration>,        // Time-to-live for online serving
    pub batch_source: Option<DataSource>,
    pub stream_source: Option<DataSource>,
    pub online: bool,                  // Serve from online store
    pub offline: bool,                 // Serve from offline store
    pub materialization_intervals: Vec<(DateTime<Utc>, DateTime<Utc>)>,
    pub state: FeatureViewState,
}
```

## Usage

```python
from ofs import FeatureView, Feature

fv = FeatureView("user_features")
fv.add_entity("user")
fv.add_feature(Feature("age", 3))      # ValueType.Int32
fv.add_feature(Feature("gender", 2))   # ValueType.String
fv.set_ttl_secs(86400)                 # 24 hour TTL

store.apply_feature_view(fv, project="my_project")
```

## States

Feature views have a lifecycle with the following states:

- `Created` — Defined but not yet materialized
- `Generated` — Offline data available
- `Materializing` — Currently being materialized to online store
- `AvailableOnline` — Ready for online serving

## TTL (Time-to-Live)

The TTL controls how long features are considered valid in the online store.
When reading features, values older than the TTL are filtered out.
