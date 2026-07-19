"""OpenFeatureStore - Offline+Online Feature Store with point-in-time correctness.

A high-performance feature store built in Rust, providing:
- Point-in-time correct feature retrieval via ASOF joins
- SQLite-backed registry and online store
- DuckDB-powered offline store
- Materialization engine for batch→online serving

Usage:
    from ofs import FeatureStore

    store = FeatureStore.in_memory()
    store.apply_entity("user", join_keys=["user_id"])
    store.apply_feature_view("user_features", entities=["user"], features=["age", "gender"])
    store.materialize(start_date=1700000000, end_date=1700086400)
    result = store.online_read(entity_key={"user_id": b"123"}, features=["age"])
"""

# Re-export Rust extension module classes
from .features import (
    ValueType,
    SourceType,
    FileFormat,
    Entity,
    Feature,
    FeatureView,
    DataSource,
    DataSourceOptions,
    EntityKey,
    RepoConfig,
    FeatureService,
)
from .stores import (
    SqlRegistry,
    DuckDbOfflineStore,
    SqliteOnlineStore,
    DefaultMaterializationEngine,
)
from .feature_store import FeatureStore

__all__ = [
    "FeatureStore",
    "ValueType",
    "SourceType",
    "FileFormat",
    "Entity",
    "Feature",
    "FeatureView",
    "DataSource",
    "DataSourceOptions",
    "EntityKey",
    "RepoConfig",
    "FeatureService",
    "SqlRegistry",
    "DuckDbOfflineStore",
    "SqliteOnlineStore",
    "DefaultMaterializationEngine",
]
