# ofs-online-store

Online store implementations for low-latency feature serving.

## SqliteOnlineStore

Per-feature-view table storage with `INSERT OR REPLACE` semantics:

```rust
pub struct SqliteOnlineStore;

impl SqliteOnlineStore {
    pub fn new(pool: SqlitePool) -> Self;
    pub async fn in_memory() -> OfsResult<Self>;
}
```

### Storage Schema

```sql
CREATE TABLE IF NOT EXISTS fv__{feature_view_name} (
    entity_key   BLOB PRIMARY KEY,
    feature_name TEXT NOT NULL,
    value        BLOB,
    event_ts     TIMESTAMP,
    created_ts   TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## RedisOnlineStore

Hash-based online store with TTL support:

```rust
pub struct RedisOnlineStore;

impl RedisOnlineStore {
    pub async fn new(connection_string: &str) -> OfsResult<Self>;
}
```

### Key Format

```
{project}:{feature_view}:{entity_key_hex}
```

Each key maps to a Redis hash where field names are feature names and
values are serialized feature values.

## Common Interface

Both stores implement the `OnlineStore` trait:

```rust
impl OnlineStore for SqliteOnlineStore {
    async fn online_read(
        &self, entity_keys: Vec<EntityKey>,
        features: &[FeatureViewWithProjection],
        project: &str,
    ) -> OfsResult<OnlineReadResponse>;

    async fn online_write_batch(
        &self, data: Vec<OnlineWriteRecord>, project: &str,
    ) -> OfsResult<()>;

    async fn update(
        &self, tables_to_keep: Vec<String>, tables_to_delete: Vec<String>,
    ) -> OfsResult<()>;

    async fn teardown(&self) -> OfsResult<()>;
}
```
