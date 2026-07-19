#!/usr/bin/env bash
set -euo pipefail
# Build OpenFeatureStore documentation site + cargo API docs

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DOCS_DIR="$ROOT_DIR/docs"
CARGO_DOC_DIR="$ROOT_DIR/target/doc"
SITE_DIR="$DOCS_DIR/book"
API_DIR="$SITE_DIR/api"

echo "=== Building OpenFeatureStore Documentation ==="

# 1. Build mdbook site
echo "[1/3] Building mdbook site..."
mdbook build "$DOCS_DIR"
echo "  → $SITE_DIR"

# 2. Build cargo API docs
echo "[2/3] Building cargo API docs..."
export PATH="/tmp/protoc/bin:$PATH"
export DUCKDB_LIB_DIR=/tmp/duckdb-lib
export LD_LIBRARY_PATH=/tmp/duckdb-lib:$LD_LIBRARY_PATH
cargo doc --workspace --exclude ofs-python --no-deps 2>&1 | tail -1
echo "  → $CARGO_DOC_DIR"

# 3. Copy cargo docs into mdbook site
echo "[3/3] Copying API docs into site..."
rm -rf "$API_DIR"
mkdir -p "$API_DIR"
cp -r "$CARGO_DOC_DIR"/* "$API_DIR/"
echo "  → $API_DIR"

echo ""
echo "=== Documentation build complete ==="
echo "  Site:    file://$SITE_DIR/index.html"
echo "  API:     file://$API_DIR/ofs_core/index.html"
echo ""
echo "  To serve locally:  mdbook serve docs/"
