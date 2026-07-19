# Feature Services

A **FeatureService** aggregates multiple feature views into a serving bundle,
allowing clients to request features from multiple views in a single call.

## Definition

```rust
pub struct FeatureService {
    pub name: String,
    pub project: String,
    pub features: Vec<FeatureViewProjection>,
    pub tags: HashMap<String, String>,
    pub description: String,
    pub owner: String,
    pub logging_config: Option<LoggingConfig>,
}
```

## Usage

```python
from ofs import FeatureService

service = FeatureService("user_features_service")
# Feature views are added via projections
```

## FeatureViewProjection

A projection selects a subset of features from a feature view, with optional
alias support:

```rust
pub struct FeatureViewProjection {
    pub feature_view_name: String,
    pub feature_view_name_alias: Option<String>,
    pub feature_columns: Vec<Feature>,
    pub join_key_map: HashMap<String, String>,
    // ...
}
```
