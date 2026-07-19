# Rust SDK Overview

OpenFeatureStore is built as a Rust workspace with 17 crates. Each crate provides
a specific layer of functionality with clean trait boundaries.

## Crate Structure

| Crate | Description | Dependencies |
|---|---|---|
| `ofs-core` | Domain types, traits, errors, utilities | prost, chrono, arrow |
| `ofs-proto` | Generated protobuf definitions + gRPC stubs | prost, prost-types, tonic |
| `ofs-registry` | SQLite-based metadata registry | ofs-core, sqlx |
| `ofs-offline-store` | DuckDB query builder for offline features | ofs-core, duckdb |
| `ofs-online-store` | SQLite and Redis online serving | ofs-core, redis |
| `ofs-materialization` | Offline-to-online bridge | ofs-core, ofs-registry, ofs-offline, ofs-online |
| `ofs-config` | YAML config with env-var interpolation and secrets | serde, serde_yaml |
| `ofs-api-types` | REST API response types and validation | serde, axum |
| `ofs-observability` | Metrics, health checks, audit logging, structured logging | prometheus, tracing |
| `ofs-serving` | Axum REST + tonic gRPC server with middleware stack | all above, axum, tonic |
| `ofs-remote-store` | S3/GCS/Azure remote storage via object_store | object_store |
| `ofs-backfill` | Resumable backfill engine with progress tracking | ofs-core, ofs-registry |
| `ofs-stream-ingest` | WAL, DLQ, push + Kafka streaming ingestion | ofs-core, rdkafka (optional) |
| `ofs-cache` | Multi-tier cache (moka L1, Redis L2) + CacheWarmer | moka, redis |
| `ofs-auth` | Authentication providers + RBAC | jsonwebtoken, reqwest |
| `ofs-python` | Python bindings via PyO3 | all of the above, pyo3 |

## Traits

The four core traits define the store interfaces:

```rust
#[async_trait]
pub trait Registry: Send + Sync {
    async fn apply_entity(&self, entity: &Entity, project: &str) -> OfsResult<()>;
    async fn get_entity(&self, name: &str, project: &str) -> OfsResult<Option<Entity>>;
    async fn list_entities(&self, project: &str) -> OfsResult<Vec<Entity>>;
    async fn delete_entity(&self, name: &str, project: &str) -> OfsResult<()>;
    // ... feature views, services, data sources, materialization
}

#[async_trait]
pub trait OfflineStore: Send + Sync {
    async fn get_historical_features(
        &self, entity_df: EntityDataFrame,
        features: Vec<FeatureViewWithProjection>,
        config: &RepoConfig,
    ) -> OfsResult<RetrievalJob>;

    async fn pull_features(
        &self, feature_view: &FeatureView,
        start_date: DateTime<Utc>, end_date: DateTime<Utc>,
    ) -> OfsResult<RetrievalJob>;
}

#[async_trait]
pub trait OnlineStore: Send + Sync {
    async fn online_read(
        &self, entity_keys: Vec<EntityKey>,
        features: &[FeatureViewWithProjection],
        project: &str,
    ) -> OfsResult<OnlineReadResponse>;

    async fn online_write_batch(
        &self, data: Vec<OnlineWriteRecord>, project: &str,
    ) -> OfsResult<()>;
}

#[async_trait]
pub trait MaterializationEngine: Send + Sync {
    async fn materialize(
        &self, start_date: DateTime<Utc>, end_date: DateTime<Utc>,
        feature_views: Option<Vec<String>>, project: &str,
        full_feature_names: bool,
    ) -> OfsResult<()>;
}
```

## Async Runtime

All traits use async methods via `async-trait`. The Python bindings use a
global Tokio runtime with `block_on` for synchronous Python calls.
