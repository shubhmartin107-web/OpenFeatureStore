"""Integration tests for the Python SDK."""

import sys
import os

# Ensure LD_LIBRARY_PATH includes DuckDB
if "LD_LIBRARY_PATH" not in os.environ:
    duckdb_path = os.environ.get("DUCKDB_LIB_DIR", "/tmp/duckdb-lib")
    os.environ["LD_LIBRARY_PATH"] = duckdb_path

from ofs import (
    FeatureStore,
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
    SqlRegistry,
    DuckDbOfflineStore,
    SqliteOnlineStore,
    DefaultMaterializationEngine,
)


def test_imports():
    """All types are importable."""
    assert ValueType.String == ValueType.String
    assert SourceType.BatchFile == SourceType.BatchFile
    assert FileFormat.Parquet == FileFormat.Parquet


def test_value_type():
    assert ValueType.String.is_primitive()
    assert ValueType.StringList.is_list()
    assert ValueType.StringSet.is_set()
    assert not ValueType.Int32.is_list()
    assert ValueType.from_i32(2) == ValueType.String
    assert ValueType.from_i32(999) is None
    # __str__ on the Rust side returns uppercase
    assert ValueType.String.__str__() in ("String", "STRING")


def test_entity():
    e = Entity("user", ["user_id"])
    assert e.name == "user"
    assert e.join_keys == ["user_id"]
    e.set_description("A user entity")
    assert e.description == "A user entity"
    e.set_owner("data-team")
    assert repr(e) == "Entity(name=user)"


def test_feature():
    f = Feature("age", ValueType.Int32)
    assert f.name == "age"
    assert f.value_type == ValueType.Int32


def test_feature_view():
    fv = FeatureView("user_features")
    fv.add_entity("user")
    fv.add_feature(Feature("age", ValueType.Int32))
    fv.add_feature(Feature("gender", ValueType.String))
    fv.set_ttl_secs(86400)
    assert fv.name == "user_features"
    assert fv.entities == ["user"]
    assert len(fv.features) == 2
    assert fv.features[0].name == "age"


def test_entity_key():
    ek = EntityKey(["user_id"])
    ek.add_value("user_id", b"12345", ValueType.String)
    assert ek.join_keys == ["user_id", "user_id"]
    serialized = ek.serialize()
    assert isinstance(serialized, bytes)
    deserialized = EntityKey.deserialize(serialized)
    assert deserialized is not None


def test_repo_config():
    config = RepoConfig()
    config.project = "my_project"
    assert config.project == "my_project"


def test_feature_service():
    fs = FeatureService("model_v1")
    assert fs.name == "model_v1"


def test_feature_store_entity_crud():
    store = FeatureStore.in_memory("test")
    store.apply_entity("user", ["user_id"])
    store.apply_entity("item", ["item_id"])

    entities = store.list_entities()
    assert len(entities) == 2
    names = {e["name"] for e in entities}
    assert names == {"user", "item"}


def test_feature_store_feature_view_crud():
    store = FeatureStore.in_memory("test")
    store.apply_feature_view("features_a", ["user"], ["age", "gender"])
    store.apply_feature_view("features_b", ["item"], ["price", "category"])

    fvs = store.list_feature_views()
    assert len(fvs) == 2
    names = {fv["name"] for fv in fvs}
    assert names == {"features_a", "features_b"}

    fv = [fv for fv in fvs if fv["name"] == "features_a"][0]
    assert "age" in fv["features"]
    assert "gender" in fv["features"]


def test_online_write_read():
    store = FeatureStore.in_memory("test")
    store.apply_entity("user", ["user_id"])
    store.apply_feature_view("user_features", ["user"], ["age", "gender"])

    store.online_write(
        {"user_id": b"user_001"},
        {"age": b"28", "gender": b"female"},
        "user_features",
    )

    result = store.online_read(
        {"user_id": b"user_001"},
        ["age", "gender"],
        "user_features",
    )
    assert result == [b"28", b"female"], f"Got {result}"


def test_online_read_missing_entity():
    store = FeatureStore.in_memory("test")
    store.apply_entity("user", ["user_id"])
    store.apply_feature_view("user_features", ["user"], ["age"])

    result = store.online_read(
        {"user_id": b"nonexistent"},
        ["age"],
        "user_features",
    )
    assert result == [None], f"Got {result}"


def test_materialization_engine():
    registry = SqlRegistry.in_memory()
    offline = DuckDbOfflineStore()
    online = SqliteOnlineStore.in_memory()

    engine = DefaultMaterializationEngine.create(registry, offline, online, "test")
    assert engine is not None

    # Materialize with no feature views should fail gracefully
    # (this will try to query DuckDB which works with in-memory)
    try:
        engine.materialize(1700000000, 1700086400, "test")
    except RuntimeError as e:
        # Expected: no feature views registered or no data sources
        assert "No feature" in str(e) or "Table" in str(e)


def test_project_isolation():
    store_a = FeatureStore.in_memory("project_a")
    store_b = FeatureStore.in_memory("project_b")

    store_a.apply_entity("user_a", ["id_a"])
    store_b.apply_entity("user_b", ["id_b"])

    assert len(store_a.list_entities()) == 1
    assert store_a.list_entities()[0]["name"] == "user_a"
    assert len(store_b.list_entities()) == 1
    assert store_b.list_entities()[0]["name"] == "user_b"
