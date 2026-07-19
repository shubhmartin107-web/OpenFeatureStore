# Python SDK Examples

## Full Workflow

```python
"""End-to-end feature store workflow."""
from ofs import FeatureStore, ValueType

# 1. Initialize
store = FeatureStore.in_memory(project="recommendation")

# 2. Register entities
store.apply_entity("user", join_keys=["user_id"])
store.apply_entity("item", join_keys=["item_id"])

# 3. Register feature views
store.apply_feature_view(
    "user_features",
    entities=["user"],
    features=["age", "gender", "country", "signup_date"],
    ttl_secs=86400 * 7,  # 1 week
)

store.apply_feature_view(
    "item_features",
    entities=["item"],
    features=["category", "price", "rating"],
    ttl_secs=86400 * 30,  # 30 days
)

# 4. List resources
for entity in store.list_entities():
    print(f"Entity: {entity['name']}, keys: {entity['join_keys']}")

for fv in store.list_feature_views():
    print(f"FeatureView: {fv['name']}, features: {fv['features']}")

# 5. Materialize (in production, data would come from DuckDB offline store)
store.materialize(start_date=1700000000, end_date=1700086400)

# 6. Write features directly for testing
store.online_write(
    entity_keys={"user_id": b"user_001"},
    values={
        "age": b"28",
        "gender": b"female",
        "country": b"US",
        "signup_date": b"2023-01-15",
    },
    feature_view_name="user_features",
)

# 7. Read features online
features = store.online_read(
    entity_keys={"user_id": b"user_001"},
    features=["age", "gender", "country"],
    feature_view_name="user_features",
)

print(f"User features: {features}")
# Output: [b'28', b'female', b'US']
```

## Using the CLI

```bash
# Initialize
ofs init --project demo

# Apply entity
ofs apply-entity user --join-keys user_id

# Apply feature view
ofs apply-feature-view user_features \
    --entities user \
    --features age,gender,country

# List resources
ofs list-entities
ofs list-feature-views

# Materialize
ofs materialize --start-date 1700000000 --end-date 1700086400

# Incremental materialization
ofs incremental --end-date 1700090000
```

## MCP Server

```bash
# Start MCP server (reads JSON from stdin, writes to stdout)
echo '{"tool": "init", "args": {"project": "demo"}}' | ofs-mcp
echo '{"tool": "apply_entity", "args": {"name": "user", "join_keys": ["user_id"]}}' | ofs-mcp
echo '{"tool": "list_entities", "args": {}}' | ofs-mcp
```
