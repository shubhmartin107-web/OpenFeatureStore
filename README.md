# OpenFeatureStore

**Feature store with point-in-time correctness** — offline/online serving, stream ingestion, and materialization, built in Rust with Python bindings.

![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)
[![CI](https://github.com/shubhmartin107-web/OpenFeatureStore/actions/workflows/ci.yml/badge.svg)](https://github.com/shubhmartin107-web/OpenFeatureStore/actions/workflows/ci.yml)
![License](https://img.shields.io/github/license/shubhmartin107-web/OpenFeatureStore)
[![Docs](https://img.shields.io/badge/docs-mdBook-blue)](https://shubhmartin107-web.github.io/OpenFeatureStore)

---

## Features

- **REST + gRPC serving** — dual-protocol feature serving with Swagger UI
- **Point-in-time correctness** — ASOF joins for historical feature retrieval
- **Multiple backends** — SQLite/PostgreSQL (registry), SQLite/Redis (online), DuckDB (offline)
- **Stream ingestion** — HTTP push and Kafka with deduplication & dead-letter queue
- **Materialization** — batch-scoring from offline to online store
- **Backfill** — scheduled or manual historical feature computation
- **Multi-layer cache** — L1 (in-memory) + L2 (Redis) with warming
- **Auth** — noop, API key, JWT, or OIDC; RBAC per project
- **Data lifecycle** — TTL-based cleanup of stale features
- **MCP server** — Model Context Protocol support for AI agents

## Architecture

```
┌──────────┐   ┌──────────┐   ┌──────────────┐
│  REST    │   │  gRPC    │   │  Stream Ingest│
│  :8080   │   │  :8081   │   │  (HTTP/Kafka) │
└────┬─────┘   └────┬─────┘   └──────┬───────┘
     └──────────────┼────────────────┘
                    ▼
         ┌──────────────────┐
         │   Auth Layer     │
         │ (noop/api_key/   │
         │  jwt/oidc + RBAC)│
         └────────┬─────────┘
                  ▼
         ┌──────────────────┐
         │   Online Store   │
         │  SQLite / Redis  │
         └────────┬─────────┘
                  │
         ┌────────▼─────────┐    ┌─────────────────┐
         │    Registry      │    │  Offline Store   │
         │ SQLite/PostgreSQL│    │     DuckDB       │
         └─────────────────┘    └────────┬─────────┘
                                         │
                              ┌──────────▼──────────┐
                              │ Materialization /    │
                              │ Backfill Engine      │
                              └─────────────────────┘
```

## Quick Start

### Prerequisites

- Rust toolchain (edition 2024)
- `protoc` (Protocol Buffers compiler)
- DuckDB shared library (for DuckDB offline store)

### Run with SQLite

```bash
# Clone and build
git clone https://github.com/shubhmartin107-web/OpenFeatureStore.git
cd OpenFeatureStore
cargo build --release --workspace --exclude ofs-python

# Start with default config
cp ofs.example.yaml ofs.yaml
cargo run --release -p ofs-server
```

```bash
# Write a feature
curl -X POST http://localhost:8080/v1/features:write-online \
  -H "Content-Type: application/json" \
  -d '{
    "project": "default",
    "feature_view": "driver_stats",
    "entity_key": "driver:1001",
    "features": {"avg_daily_trips": 42}
  }'

# Read it back
curl -X POST http://localhost:8080/v1/features:get-online \
  -H "Content-Type: application/json" \
  -d '{
    "features": ["driver_stats__avg_daily_trips"],
    "entities": {"driver_id": ["1001"]}
  }'
```

### Docker

```bash
docker compose up -d
```

## Project Structure

| Crate | Description |
|---|---|
| `ofs-core` | Core types, traits, EntityKey, ValueType |
| `ofs-config` | YAML config with env-var interpolation |
| `ofs-registry` | SQLite + PostgreSQL metadata registry |
| `ofs-online-store` | SQLite + Redis online feature store |
| `ofs-offline-store` | DuckDB offline store (SQL query builder) |
| `ofs-serving` | REST + gRPC serving with auth middleware |
| `ofs-server` | Binary entry point wiring all crates |
| `ofs-auth` | Authentication providers + RBAC |
| `ofs-lifecycle` | TTL-based data lifecycle management |
| `ofs-materialization` | Batch materialization engine |
| `ofs-backfill` | Historical feature backfill jobs |
| `ofs-stream-ingest` | HTTP push + Kafka ingestion |
| `ofs-cache` | L1 (moka) + L2 (Redis) caching |
| `ofs-observability` | Prometheus metrics, health checks, audit |
| `ofs-remote-store` | S3/GCS/Azure Blob remote storage |
| `ofs-python` | Python bindings via PyO3 |
| `ofs-proto` | Feast-compatible protobuf definitions |
| `ofs-api-types` | Shared API types and validation |

## Documentation

Full documentation is available as an [mdBook](docs/book/index.html). Source files are in [`docs/src/`](docs/src/).

- [Quick Start](docs/src/quick_start.md)
- [Installation](docs/src/installation.md)
- [Architecture Overview](docs/src/architecture/overview.md)
- [Configuration](docs/src/operations/configuration.md)
- [REST API Reference](crates/ofs-serving/src/rest/features.rs) (or visit `/docs` on a running server)
- [Building & Testing](docs/src/development/building.md)

## Development

```bash
# Check & lint
cargo check --workspace --exclude ofs-python
cargo clippy --workspace --exclude ofs-python -- -D warnings

# Test
cargo test --workspace --exclude ofs-python

# Format
cargo fmt --all
```

Requires `protoc` at build time. A bootstrap script is provided:

```bash
./scripts/bootstrap.sh
```

## License

Apache 2.0
