#!/usr/bin/env bash
# Compute the auditwheel hash of libduckdb.so and create a symlink so that
# subsequent `maturin build` invocations can find the library by its hash name.
#
# The hash is the first 8 hex chars of SHA-256 of the .so file.
# This is deterministic for a given file.

DUCKDB_DIR="${1:-/tmp/duckdb-lib}"
LIB="$DUCKDB_DIR/libduckdb.so"

if [ ! -f "$LIB" ]; then
    echo "ERROR: $LIB not found. Set DUCKDB_LIB_DIR or pass path as argument." >&2
    exit 1
fi

HASH=$(sha256sum "$LIB" | cut -c1-8)
HASHED_LIB="$DUCKDB_DIR/libduckdb-${HASH}.so"

if [ ! -L "$HASHED_LIB" ] && [ ! -f "$HASHED_LIB" ]; then
    ln -s libduckdb.so "$HASHED_LIB"
    echo "Created symlink: $HASHED_LIB -> libduckdb.so"
else
    echo "Symlink exists: $HASHED_LIB"
fi
