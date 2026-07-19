"""High-level FeatureStore that orchestrates registry, stores, and materialization."""

import time
from typing import Any, Dict, List, Optional

from .features import (
    ValueType,
    Entity,
    Feature,
    FeatureView,
    EntityKey,
    RepoConfig,
)
from .stores import (
    SqlRegistry,
    DuckDbOfflineStore,
    SqliteOnlineStore,
    DefaultMaterializationEngine,
)


class FeatureStore:
    """High-level feature store combining registry, offline/online stores, and materialization.

    Provides a unified API for feature registration, materialization, and online serving.

    Example:
        store = FeatureStore.in_memory()
        store.apply_entity("user", join_keys=["user_id"])
        store.apply_feature_view("user_features",
            entities=["user"], features=["age", "gender"])
        store.materialize(start_date=1700000000, end_date=1700086400)
        result = store.online_read({"user_id": b"123"}, ["age"])
    """

    def __init__(
        self,
        registry: SqlRegistry,
        offline_store: DuckDbOfflineStore,
        online_store: SqliteOnlineStore,
        project: str = "default",
    ):
        self._registry = registry
        self._offline_store = offline_store
        self._online_store = online_store
        self._project = project
        self._engine = DefaultMaterializationEngine.create(
            registry, offline_store, online_store, project
        )

    @classmethod
    def in_memory(cls, project: str = "default") -> "FeatureStore":
        """Create a FeatureStore with in-memory registry and online store."""
        registry = SqlRegistry.in_memory()
        offline_store = DuckDbOfflineStore()
        online_store = SqliteOnlineStore.in_memory()
        return cls(registry, offline_store, online_store, project)

    def apply_entity(self, name: str, join_keys: List[str], **kwargs) -> None:
        """Register an entity."""
        entity = Entity(name, join_keys)
        if "description" in kwargs:
            entity.set_description(kwargs["description"])
        if "owner" in kwargs:
            entity.set_owner(kwargs["owner"])
        self._registry.apply_entity(entity, self._project)

    def apply_feature_view(
        self,
        name: str,
        entities: List[str],
        features: List[str],
        ttl_secs: Optional[int] = None,
    ) -> None:
        """Register a feature view."""
        fv = FeatureView(name)
        for entity_name in entities:
            fv.add_entity(entity_name)
        for feature_name in features:
            fv.add_feature(Feature(feature_name, ValueType.String))
        if ttl_secs is not None:
            fv.set_ttl_secs(ttl_secs)
        self._registry.apply_feature_view(fv, self._project)

    def list_entities(self) -> List[Dict[str, Any]]:
        """List all registered entities."""
        entities = self._registry.list_entities(self._project)
        return [
            {
                "name": e.name,
                "join_keys": e.join_keys,
                "description": e.description,
            }
            for e in entities
        ]

    def list_feature_views(self) -> List[Dict[str, Any]]:
        """List all registered feature views."""
        fvs = self._registry.list_feature_views(self._project)
        return [
            {
                "name": fv.name,
                "entities": fv.entities,
                "features": [f.name for f in fv.features],
            }
            for fv in fvs
        ]

    def materialize(self, start_date: float, end_date: float) -> None:
        """Materialize features from offline to online store."""
        self._engine.materialize(start_date, end_date, self._project)

    def materialize_incremental(self, end_date: float) -> None:
        """Incrementally materialize features since last materialization."""
        self._engine.materialize_incremental(end_date, self._project)

    def online_read(
        self,
        entity_keys: Dict[str, bytes],
        features: List[str],
        feature_view_name: str = "default",
    ) -> List[Optional[bytes]]:
        """Read feature values for an entity from the online store."""
        ek = EntityKey(list(entity_keys.keys()))
        for k, v in entity_keys.items():
            ek.add_value(k, v, ValueType.String)
        return self._online_store.online_read(
            ek, feature_view_name, features, self._project
        )

    def online_write(
        self,
        entity_keys: Dict[str, bytes],
        values: Dict[str, bytes],
        feature_view_name: str = "default",
    ) -> None:
        """Write feature values to the online store."""
        ek = EntityKey(list(entity_keys.keys()))
        for k, v in entity_keys.items():
            ek.add_value(k, v, ValueType.String)
        self._online_store.online_write(
            ek, values, feature_view_name, self._project
        )
