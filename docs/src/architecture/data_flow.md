# Data Flow

This page describes how data flows through the feature store during materialization and serving.

## Materialization Flow

The materialization engine bridges the offline and online stores:

```
┌──────────┐    pull_features()     ┌──────────┐
│  DuckDB   │ ◄─────────────────── │ Registry  │
│ Offline   │                       │ (SQLite)  │
│ Store    │                        └─────┬─────┘
└────┬─────┘                              │
     │                                     │ get_feature_views()
     │ SQL query                           │
     ▼                                     ▼
┌──────────┐                    ┌──────────────────┐
│ DuckDB   │                    │ Materialization   │
│ Query    │                    │ Engine            │
│ Result   │ ──── rows ───────► │                   │
└──────────┘                    └────────┬──────────┘
                                         │
                               online_write_batch()
                                         │
                                         ▼
                                  ┌──────────┐
                                  │  SQLite   │
                                  │  Online   │
                                  │  Store    │
                                  └──────────┘
```

### Steps

1. Registry returns all feature views for a project
2. Engine calls `offline_store.pull_features(fv, start, end)` → gets SQL query
3. Engine executes the DuckDB SQL query directly
4. For each row, converts to `OnlineWriteRecord`
5. Writes records to online store in batches of 100

## Serving Flow

When features are requested for online serving:

```
Client ──► online_read(entity_keys, features)
                  │
                  ▼
         ┌────────────────┐
         │  Online Store   │
         │  (SQLite/Redis) │
         └───────┬────────┘
                 │
                 ▼
         Feature values returned
```

### Point-in-Time Correctness

The offline store uses ASOF joins to ensure features are joined at the correct
point in time:

```sql
SELECT *
FROM entity_dataframe ef
ASOF JOIN feature_table fv
  ON ef.entity_key = fv.entity_key
 AND ef.timestamp >= fv.timestamp
```

This guarantees that for each entity row, only features that were valid at
(or before) the entity's timestamp are included — never future features.
