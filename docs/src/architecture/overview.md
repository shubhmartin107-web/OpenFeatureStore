# Architecture Overview

OpenFeatureStore follows a layered architecture inspired by Feast, with clean trait
boundaries between each component.

## Layers

### 1. Core Layer (`ofs-core`)

Defines the fundamental abstractions:

- **Types** — Domain models: `Entity`, `FeatureView`, `DataSource`, `FeatureService`,
  `EntityKey`, `RepoConfig`
- **Traits** — Store interfaces: `Registry`, `OfflineStore`, `OnlineStore`,
  `MaterializationEngine`
- **Value Types** — Feast-compatible type system (`ValueType`, `FeastType`)
- **Entity Key** — Serialization/deserialization of composite entity keys (Feast v3 format)
- **Errors** — Unified error types across all crates

### 2. Registry Layer (`ofs-registry`)

Manages feature store metadata:

- SQLite-backed persistence of entities, feature views, data sources, feature services
- Full CRUD operations scoped by project
- Materialization interval tracking
- Proto-based serialization (Feast SQL registry pattern)

### 3. Offline Store (`ofs-offline-store`)

Handles batch feature computation:

- DuckDB-powered SQL query builder
- ASOF JOIN for point-in-time correct feature retrieval
- Returns `RetrievalJob` with SQL query and schema metadata

### 4. Online Store (`ofs-online-store`)

Low-latency feature serving:

- **SQLiteOnlineStore** — Per-feature-view tables with `INSERT OR REPLACE`
- **RedisOnlineStore** — Hash-based storage with TTL support

### 5. Materialization Engine (`ofs-materialization`)

Bridges offline and online stores:

- Pulls feature data from offline store (DuckDB)
- Iterates results row-by-row
- Writes to online store in batches of 100

## Crate Dependency Graph

```
ofs-proto (protobuf definitions)
    ↓
ofs-core (traits, types, errors)
    ↓
ofs-registry → ofs-core
ofs-offline-store → ofs-core
ofs-online-store → ofs-core
    ↓
ofs-materialization → ofs-core + ofs-registry + ofs-offline + ofs-online
    ↓
ofs-python (PyO3 bindings) → all of the above
```

## Key Design Decisions

- **Trait-based** — All stores implement async traits for testability and swap-ability
- **Proto serialization** — Registry stores serialized `Registry` protobuf blobs in SQLite,
  matching the Feast SQL registry pattern
- **Deferred execution** — Offline store returns `RetrievalJob` with SQL string,
  deferring DuckDB execution to the materialization engine
- **Batch writes** — Online store writes are batched in chunks of 100 records
