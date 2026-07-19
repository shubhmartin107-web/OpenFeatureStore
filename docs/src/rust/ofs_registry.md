# ofs-registry

SQLite-backed registry implementing the `Registry` trait.

## SqlRegistry

```rust
pub struct SqlRegistry;

impl SqlRegistry {
    pub async fn new(pool: SqlitePool) -> OfsResult<Self>;
    pub async fn in_memory() -> OfsResult<Self>;
}
```

## Storage Format

Tables:

- `registries(project TEXT, name TEXT, last_synced INT, registry_proto_bytes BLOB)`

Each row stores a serialized `feast::core::Registry` protobuf for a project.
Domain types are converted to/from protobuf representations.

## Methods

All `Registry` trait methods are implemented:

- **Entities**: `apply_entity`, `get_entity`, `list_entities`, `delete_entity`
- **Feature Views**: `apply_feature_view`, `get_feature_view`, `list_feature_views`, `delete_feature_view`
- **Feature Services**: `apply_feature_service`, `get_feature_service`, `list_feature_services`, `delete_feature_service`
- **Data Sources**: `apply_data_source`, `get_data_source`, `list_data_sources`, `delete_data_source`
- **On-Demand FVs**: `apply_on_demand_feature_view`, `list_on_demand_feature_views`
- **Materialization**: `apply_materialization`, `get_materialization_intervals`
- **Persistence**: `commit`

## Usage

```rust
use ofs_registry::SqlRegistry;
use ofs_core::types::Entity;

let registry = SqlRegistry::in_memory().await?;
let entity = Entity::new("user", vec!["user_id".to_string()]);
registry.apply_entity(&entity, "my_project").await?;
```
