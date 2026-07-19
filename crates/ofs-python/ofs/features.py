"""Core feature store domain types."""

try:
    from ofs._rust import (
        ValueType,
        SourceType,
        FileFormat,
        Entity as _Entity,
        Feature as _Feature,
        FeatureView as _FeatureView,
        DataSource as _DataSource,
        DataSourceOptions,
        EntityKey,
        RepoConfig,
        FeatureService,
    )
except ImportError:
    import sys
    print("OpenFeatureStore native module not found. Build with: maturin develop --release", file=sys.stderr)
    raise

# Re-export with original names
Entity = _Entity
Feature = _Feature
FeatureView = _FeatureView
DataSource = _DataSource

__all__ = [
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
]
