# Storage Layer

## Registry Storage (SQLite)

The registry stores metadata in a SQLite database using the Feast SQL registry pattern:

- Single `registries` table with columns: `project`, `name`, `last_synced`, `registry_proto_bytes`
- Each row stores a serialized protobuf `Registry` for a project
- On read, the proto is deserialized and domain types are extracted
- On write, the proto is updated and re-serialized

```
Table: registries
┌─────────┬──────────┬──────────────┬──────────────────────┐
│ project │  name    │ last_synced  │ registry_proto_bytes │
├─────────┼──────────┼──────────────┤──────────────────────┤
│ default │ default  │ 1700000000   │ <protobuf blob>      │
│ prod    │ default  │ 1700000000   │ <protobuf blob>      │
└─────────┴──────────┴──────────────┴──────────────────────┘
```

## Offline Store (DuckDB)

The offline store does not maintain persistent storage. Instead:

- It builds SQL queries that reference external data sources (Parquet files, etc.)
- The actual data querying is deferred to the materialization engine
- DuckDB's ASOF JOIN capability provides point-in-time correctness

```sql
-- Example generated query
SELECT
  entity_df.entity_key,
  entity_df.event_timestamp,
  feature_table.feature_value
FROM parquet_read('/data/entities.parquet') entity_df
ASOF JOIN parquet_read('/data/features.parquet') feature_table
  ON entity_df.entity_key = feature_table.entity_key
 AND entity_df.event_timestamp >= feature_table.event_timestamp
```

## Online Store

### SQLite Online Store

Per-feature-view tables with row-level storage:

```sql
CREATE TABLE IF NOT EXISTS fv__user_features (
    entity_key   BLOB PRIMARY KEY,
    feature_name TEXT NOT NULL,
    value        BLOB,
    event_ts     TIMESTAMP,
    created_ts   TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

- `INSERT OR REPLACE` semantics for upserts
- `entity_key` is the serialized Feast v3 entity key

### Redis Online Store

Hash-based storage with entity key as the key:

```
Redis key format: {project}:{feature_view}:{entity_key_hex}

Hash fields:
  feature_name_1 ──► serialized_value_1
  feature_name_2 ──► serialized_value_2
  ...
```

- Configurable TTL for each feature view
- Connection pooling via `redis-rs` connection manager
