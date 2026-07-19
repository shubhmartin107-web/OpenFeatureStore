# Installation

## From Source

### Prerequisites

- **Rust toolchain**: Install via [rustup](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup default stable
  ```
- **Protobuf compiler** (protoc): Required for code generation
  ```bash
  # Ubuntu/Debian
  sudo apt install protobuf-compiler

  # macOS
  brew install protobuf
  ```
- **DuckDB library**: Either system-installed or compiled via `bundled` feature
  ```bash
  # Ubuntu/Debian (system library, faster build)
  sudo apt install libduckdb-dev

  # Or use bundled (compiles DuckDB from source, ~10min)
  # Just build without --no-default-features
  ```

### Build Workspace

```bash
git clone https://github.com/anomalyco/openfeaturestore.git
cd openfeaturestore

# Build all crates
cargo build --workspace

# Run tests (requires DuckDB library)
export DUCKDB_LIB_DIR=/path/to/duckdb/lib
export LD_LIBRARY_PATH=$DUCKDB_LIB_DIR:$LD_LIBRARY_PATH
cargo test --workspace
```

### Python SDK

```bash
cd crates/ofs-python
pip install maturin

# Development install
maturin develop --release

# Or build a wheel
maturin build --release
pip install target/wheels/openfeaturestore-*.whl
```

## Docker

```dockerfile
FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/ofs /usr/local/bin/
CMD ["ofs"]
```

## Verify Installation

```bash
# Check Rust crates
cargo doc --workspace --no-deps --open

# Check Python package
python -c "import ofs; print(ofs.__version__)"

# Check CLI
ofs --help
```
