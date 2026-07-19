# FeatureStore API

The `FeatureStore` class provides a unified high-level API.

## Constructor

```python
FeatureStore(registry, offline_store, online_store, project="default")
```

### Class Methods

**`FeatureStore.in_memory(project="default")`**

Creates a feature store with in-memory registry and online store, suitable for
development and testing.

```python
from ofs import FeatureStore
store = FeatureStore.in_memory(project="demo")
```

## Methods

### Entity Management

**`apply_entity(name, join_keys, description=None, owner=None)`**

Register an entity.

| Parameter | Type | Description |
|---|---|---|
| `name` | `str` | Entity name |
| `join_keys` | `List[str]` | Columns used to identify the entity |
| `description` | `str` (optional) | Human-readable description |
| `owner` | `str` (optional) | Owner name |

**`list_entities()`**

Returns a list of entity dicts with `name`, `join_keys`, `description`.

### Feature View Management

**`apply_feature_view(name, entities, features, ttl_secs=None)`**

Register a feature view.

| Parameter | Type | Description |
|---|---|---|
| `name` | `str` | Feature view name |
| `entities` | `List[str]` | Referenced entity names |
| `features` | `List[str]` | Feature names (currently all String type) |
| `ttl_secs` | `int` (optional) | Time-to-live in seconds |

**`list_feature_views()`**

Returns a list of feature view dicts with `name`, `entities`, `features`.

### Materialization

**`materialize(start_date, end_date)`**

Materialize features from the offline store to the online store.

| Parameter | Type | Description |
|---|---|---|
| `start_date` | `float` | Start Unix timestamp |
| `end_date` | `float` | End Unix timestamp |

**`materialize_incremental(end_date)`**

Incrementally materialize features since the last materialization.

### Online Serving

**`online_read(entity_keys, features, feature_view_name="default")`**

Read feature values from the online store.

| Parameter | Type | Description |
|---|---|---|
| `entity_keys` | `Dict[str, bytes]` | Entity key name to value mapping |
| `features` | `List[str]` | Feature names to retrieve |
| `feature_view_name` | `str` | Feature view name |

Returns `List[Optional[bytes]]` — feature values or `None`.

**`online_write(entity_keys, values, feature_view_name="default")`**

Write feature values to the online store.

| Parameter | Type | Description |
|---|---|---|
| `entity_keys` | `Dict[str, bytes]` | Entity key name to value mapping |
| `values` | `Dict[str, bytes]` | Feature name to serialized value mapping |
| `feature_view_name` | `str` | Feature view name |
