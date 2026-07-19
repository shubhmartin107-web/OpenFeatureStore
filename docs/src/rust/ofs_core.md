# ofs-core

The core crate defining all domain types, traits, and utilities used by the
entire feature store.

## Key Modules

### `types` Module

Domain models:

- `Entity` — A domain object with features
- `FeatureView` — Group of features from a data source
- `FeatureService` — Bundle of feature views for serving
- `DataSource` — Physical data source definition
- `EntityKey` — Composite key for entity identification
- `RepoConfig` — Repository configuration
- `OnlineWriteRecord` — Record for batch writes
- `OnlineReadResponse` — Response for online reads
- `RetrievalJob` — Deferred query result

### `traits` Module

Async trait definitions:

- `Registry` — Metadata CRUD operations
- `OfflineStore` — Historical and pull-based feature retrieval
- `OnlineStore` — Low-latency feature serving
- `MaterializationEngine` — Offline-to-online bridge

### `value_type` Module

- `ValueType` enum (49 variants)
- `FeastType` with `Primitive`, `Array`, `Set`, `Struct` variants
- `PrimitiveFeastType` enum
- Conversion to/from Arrow data types

### `entity_key` Module

- `serialize_entity_key_v3()` — Feast v3 format serialization
- `deserialize_entity_key_v3()` — Feast v3 format deserialization
- `serialize_entity_key_prefix()` — Prefix for key scanning

### `errors` Module

Unified error type:

- `OfsError` — All error variants
- `OfsResult<T>` — Type alias for `Result<T, OfsError>`

### `utils` Module

- `proto_to_datetime()` / `datetime_to_proto()` — Chrono-to-prost conversion
- `duration_to_proto()` / `proto_to_duration()` — Duration conversion

## Re-exports

All public types are re-exported from the crate root via `pub use *` on each module.
