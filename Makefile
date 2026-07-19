.PHONY: all build test test-rust test-python lint docs clean bootstrap

# Default target
all: build test lint docs

# Bootstrap environment (requires protoc + DuckDB)
bootstrap:
	@bash scripts/bootstrap.sh

# Build Rust crates (exclude Python bindings — built separately)
build:
	cargo build --workspace --exclude ofs-python

# Build release
build-release:
	cargo build --release --workspace --exclude ofs-python

# Run all Rust tests
test-rust:
	cargo test --workspace --exclude ofs-python

# Run Python tests
test-python:
	cd crates/ofs-python && python -m pytest tests/

# Run all tests
test: test-rust test-python

# Clippy
lint:
	cargo clippy --workspace --exclude ofs-python -- -D warnings

# Build documentation
docs:
	@bash scripts/build-docs.sh

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/maturin target/wheels

# Build Python wheel
wheel:
	@bash scripts/ensure-duckdb-symlink.sh
	cd crates/ofs-python && maturin build --release

# Install Python package in development mode
dev-install:
	@bash scripts/ensure-duckdb-symlink.sh
	cd crates/ofs-python && maturin develop --release
