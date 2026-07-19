# Python SDK — Getting Started

The Python SDK provides native bindings to the Rust feature store via PyO3.

## Installation

```bash
# From source
cd crates/ofs-python
pip install maturin
maturin develop --release

# Or install the wheel
maturin build --release
pip install target/wheels/openfeaturestore-*.whl
```

## Quickstart

### FeatureStore (High-Level API)

```python
from ofs import FeatureStore

# Create an in-memory feature store
store = FeatureStore.in_memory(project="demo")

# Register an entity
store.apply_entity("user", join_keys=["user_id"])

# Register a feature view with features
store.apply_feature_view(
    "user_features",
    entities=["user"],
    features=["age", "gender", "signup_date"],
    ttl_secs=86400,  # 24 hours
)

# List resources
print(store.list_entities())
print(store.list_feature_views())

# Materialize features
store.materialize(start_date=1700000000, end_date=1700086400)

# Read features online
result = store.online_read(
    entity_keys={"user_id": b"12345"},
    features=["age", "gender"],
    feature_view_name="user_features"
)
print(result)  # [b'25', b'male']
```

### Direct Store Access

```python
from ofs import (
    SqlRegistry, DuckDbOfflineStore, SqliteOnlineStore,
    Entity, Feature, FeatureView,
    DefaultMaterializationEngine,
)

# Create stores
registry = SqlRegistry.in_memory()
offline = DuckDbOfflineStore()
online = SqliteOnlineStore.in_memory()

# Work with the registry
entity = Entity("user", join_keys=["user_id"])
registry.apply_entity(entity, "demo")

# Pull features from offline store
sql = offline.pull_features("user_features", 1700000000, 1700086400)
print(f"Generated SQL: {sql}")

# Write to online store
from ofs import EntityKey
key = EntityKey(["user_id"])
key.add_value("user_id", b"12345", 2)
online.online_write(
    key, {"age": b"25"}, "user_features", "demo"
)
```
