#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== OpenFeatureStore Bootstrap ==="

# ── 1. Check Rust toolchain ──
echo "[1/5] Checking Rust toolchain..."
if ! command -v rustc &>/dev/null; then
    echo "ERROR: Rust not installed. Install via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi
echo "  rustc:  $(rustc --version)"
echo "  cargo:  $(cargo --version)"

# ── 2. Install protoc if missing ──
echo "[2/5] Checking protoc..."
if command -v /tmp/protoc/bin/protoc &>/dev/null; then
    export PATH="/tmp/protoc/bin:$PATH"
    echo "  protoc: $(protoc --version) (cached)"
elif command -v protoc &>/dev/null; then
    echo "  protoc: $(protoc --version) (system)"
else
    echo "  Downloading protoc..."
    PB_REL="https://github.com/protocolbuffers/protobuf/releases"
    curl -fsSL "$PB_REL/download/v27.0/protoc-27.0-linux-x86_64.zip" -o /tmp/protoc.zip
    unzip -q /tmp/protoc.zip -d /tmp/protoc
    rm /tmp/protoc.zip
    export PATH="/tmp/protoc/bin:$PATH"
    echo "  protoc: $(protoc --version)"
fi

# ── 3. Download DuckDB library if missing ──
echo "[3/5] Checking DuckDB library..."
DUCKDB_DIR="/tmp/duckdb-lib"
DUCKDB_LIB="$DUCKDB_DIR/libduckdb.so"
if [ -f "$DUCKDB_LIB" ]; then
    echo "  libduckdb.so found (cached)"
else
    echo "  Downloading DuckDB library v1.5.0..."
    mkdir -p "$DUCKDB_DIR"
    curl -fsSL "https://github.com/duckdb/duckdb/releases/download/v1.5.0/libduckdb-linux-amd64.zip" -o /tmp/duckdb.zip
    unzip -q /tmp/duckdb.zip -d "$DUCKDB_DIR"
    rm /tmp/duckdb.zip
    echo "  libduckdb.so: $(du -h "$DUCKDB_LIB" | cut -f1)"
fi
export DUCKDB_LIB_DIR="$DUCKDB_DIR"
export LD_LIBRARY_PATH="$DUCKDB_DIR:${LD_LIBRARY_PATH:-}"

# Create auditwheel-compatible symlink (hash-based name)
# This ensures `maturin build` finds the library on subsequent rebuilds
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
bash "$SCRIPT_DIR/ensure-duckdb-symlink.sh" "$DUCKDB_DIR"

# ── 4. Verify Cargo config ──
echo "[4/5] Checking Cargo config..."
CARGO_CONFIG="$ROOT_DIR/.cargo/config.toml"
if [ -f "$CARGO_CONFIG" ]; then
    echo "  .cargo/config.toml found"
else
    echo "  WARNING: .cargo/config.toml not found; DuckDB linking may fail"
fi

# ── 5. Install Python build tools (maturin) ──
echo "[5/5] Checking Python build tools..."
if command -v maturin &>/dev/null; then
    echo "  maturin: $(maturin --version)"
else
    echo "  Installing maturin..."
    pip3 install maturin --quiet
    echo "  maturin: $(maturin --version)"
fi

echo ""
echo "=== Bootstrap complete ==="
echo ""
echo "To build Rust crates:         cargo build --workspace --exclude ofs-python"
echo "To run Rust tests:            cargo test --workspace --exclude ofs-python"
echo "To build Python bindings:     cd crates/ofs-python && maturin develop --release"
echo "To build docs:                mdbook build docs/"
echo "To serve docs:                mdbook serve docs/"
echo ""
