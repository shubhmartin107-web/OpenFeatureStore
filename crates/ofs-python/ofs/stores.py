"""Feature store implementations."""

try:
    from ofs._rust import (
        SqlRegistry as _SqlRegistry,
        DuckDbOfflineStore as _DuckDbOfflineStore,
        SqliteOnlineStore as _SqliteOnlineStore,
        DefaultMaterializationEngine as _DefaultMaterializationEngine,
    )
except ImportError:
    import sys
    print("OpenFeatureStore native module not found. Build with: maturin develop --release", file=sys.stderr)
    raise

# Re-export with original names
SqlRegistry = _SqlRegistry
DuckDbOfflineStore = _DuckDbOfflineStore
SqliteOnlineStore = _SqliteOnlineStore
DefaultMaterializationEngine = _DefaultMaterializationEngine

__all__ = [
    "SqlRegistry",
    "DuckDbOfflineStore",
    "SqliteOnlineStore",
    "DefaultMaterializationEngine",
]
