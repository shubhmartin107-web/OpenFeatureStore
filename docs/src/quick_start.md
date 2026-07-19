# Quick Start

## Prerequisites

- Rust 1.85+ (edition 2024)
- Python 3.8+ (for Python SDK)
- DuckDB system library or `bundled` feature

## Using the Python SDK

```bash
# Install from source
cd crates/ofs-python
pip install maturin
maturin develop --release

# Use the FeatureStore
python -c "
from ofs import FeatureStore

store = FeatureStore.in_memory()

# Register an entity
store.apply_entity('user', join_keys=['user_id'])

# Register a feature view
store.apply_feature_view('user_features',
    entities=['user'],
    features=['age', 'gender', 'signup_date'])

# List what we've created
print('Entities:', store.list_entities())
print('Feature Views:', store.list_feature_views())
"
```

## Using the CLI

```bash
# Initialize a store
ofs init --project demo

# Register an entity
ofs apply-entity user --join-keys user_id

# Register a feature view
ofs apply-feature-view user_features \
    --entities user \
    --features age,gender,signup_date

# List resources
ofs list-entities
ofs list-feature-views
```

## Using Rust directly

```rust
use ofs_registry::SqlRegistry;
use ofs_core::types::Entity;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = SqlRegistry::in_memory().await?;

    let entity = Entity::new("user", vec!["user_id".to_string()]);
    registry.apply_entity(&entity, "my_project").await?;

    let found = registry.get_entity("user", "my_project").await?;
    println!("Found: {:?}", found);
    Ok(())
}
```

## Next Steps

- Learn about [Core Concepts](concepts/entities.md)
- Explore the [Python SDK](python/getting_started.md)
- Configure for [Production](operations/deployment.md)
