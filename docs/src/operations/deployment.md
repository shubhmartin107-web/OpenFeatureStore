# Deployment

## Requirements

- Linux (x86_64) or macOS (arm64/x86_64)
- Rust 1.85+ (edition 2024)
- DuckDB library (v1.5.0)
- Python 3.10+ (for Python bindings)

## Production Deployment

### Option 1: Rust Binary

Build and deploy the Rust binary with DuckDB linked:

```bash
cargo build --release --workspace --exclude ofs-python
./target/release/my-app
```

### Option 2: Python Package

Build and install the Python wheel:

```bash
cd crates/ofs-python
DUCKDB_LIB_DIR=/tmp/duckdb-lib maturin build --release
pip install target/wheels/openfeaturestore-*.whl
```

### Option 3: Docker (recommended)

```dockerfile
FROM rust:latest AS builder
RUN apt-get update && apt-get install -y protobuf-compiler
WORKDIR /app
COPY . .
RUN cargo build --release --exclude ofs-python

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libduckdb
COPY --from=builder /app/target/release/ofs /usr/local/bin/
CMD ["ofs"]
```

## Backend Configuration

### SQLite (Development)

```yaml
registry:
  type: sqlite
  path: /var/lib/featurestore/registry.db

online_store:
  type: sqlite
  path: /var/lib/featurestore/online.db

offline_store:
  type: duckdb
  path: /var/lib/featurestore/offline.db
```

### Redis (Production)

```yaml
registry:
  type: sqlite
  path: /var/lib/featurestore/registry.db

online_store:
  type: redis
  host: redis-cluster.example.com
  port: 6379
  db: 0

offline_store:
  type: duckdb
  path: s3://my-bucket/offline-store/
```
