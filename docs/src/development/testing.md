# Testing

## Running Tests

### All Rust Tests (except Python bindings)

```bash
# Standard run (takes ~30s)
cargo test --workspace --exclude ofs-python
```

### Specific Crate

```bash
# Core crate
cargo test -p ofs-core

# Registry
cargo test -p ofs-registry

# Offline store
cargo test -p ofs-offline-store

# Online store
cargo test -p ofs-online-store

# Materialization
cargo test -p ofs-materialization
```

### Test Report

After running the full test suite, verify output:

```
running 57 tests
test result: ok. 56 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
```

The one ignored test is the Redis online store test (requires a running Redis
instance).

## Test Structure

| Crate | Tests | Description |
|---|---|---|
| `ofs-core` | 34 | Types, serialization, value conversion |
| `ofs-materialization` | 5 | Engine execution, incremental materialization |
| `ofs-offline-store` | 6 | ASOF JOIN, query generation |
| `ofs-online-store` | 4 (1 ignored) | SQLite write/read, cache consistency |
| `ofs-registry` | 8 | Entity/FV/DS/FS CRUD operations |

## Python Tests

```bash
cd crates/ofs-python
maturin develop --release
python -m pytest tests/
```

## CI Testing

GitHub Actions workflow runs:

1. `cargo test --workspace --exclude ofs-python`
2. `cargo clippy --workspace --exclude ofs-python -- -D warnings`
3. `cargo fmt --check`
