# Building from Source

## Prerequisites

- **Rust**: 1.85+ (install via `rustup`)
- **Protobuf compiler**: `protoc` in PATH
- **DuckDB library**: v1.5.0 pre-compiled library
- **Python**: 3.10+ (for Python bindings)
- **maturin**: For building the Python package

## Setup

### 1. Install Protobuf Compiler

```bash
# Download protoc
PB_REL="https://github.com/protocolbuffers/protobuf/releases"
curl -LO $PB_REL/download/v27.0/protoc-27.0-linux-x86_64.zip
unzip protoc-27.0-linux-x86_64.zip -d /tmp/protoc
export PATH="/tmp/protoc/bin:$PATH"
```

### 2. Install DuckDB Library

```bash
mkdir -p /tmp/duckdb-lib
curl -LO https://github.com/duckdb/duckdb/releases/download/v1.5.0/libduckdb-linux-amd64.zip
unzip libduckdb-linux-amd64.zip -d /tmp/duckdb-lib
export DUCKDB_LIB_DIR=/tmp/duckdb-lib
export LD_LIBRARY_PATH=/tmp/duckdb-lib:$LD_LIBRARY_PATH
```

### 3. Build Rust Crate

```bash
# Build all crates except Python bindings
cargo build --workspace --exclude ofs-python

# Run tests (exclude Python crate)
cargo test --workspace --exclude ofs-python
```

### 4. Build Python Package

```bash
cd crates/ofs-python
maturin develop --release
```

## Docker Build

See [Deployment > Docker](deployment.md#option-3-docker-recommended) for a containerized build.
