"""MCP server for OpenFeatureStore integration with opencode.

Provides tools for opencode to:
- Initialize and configure feature stores
- Register entities and feature views
- Materialize features
- Query feature values

Start with:
    ofs-mcp
"""

import json
import sys
from typing import Any, Dict, List, Optional

from .feature_store import FeatureStore


class MCPFeatureStore:
    """MCP wrapper around FeatureStore for opencode integration."""

    def __init__(self, project: str = "default"):
        self._store = FeatureStore.in_memory(project=project)
        self._project = project

    def handle_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Handle an MCP tool request."""
        tool = request.get("tool", "")
        args = request.get("args", {})

        handlers = {
            "init": self._handle_init,
            "apply_entity": self._handle_apply_entity,
            "apply_feature_view": self._handle_apply_feature_view,
            "list_entities": self._handle_list_entities,
            "list_feature_views": self._handle_list_feature_views,
            "materialize": self._handle_materialize,
            "materialize_incremental": self._handle_materialize_incremental,
            "online_read": self._handle_online_read,
            "online_write": self._handle_online_write,
        }

        handler = handlers.get(tool)
        if not handler:
            return {"error": f"Unknown tool: {tool}"}

        try:
            result = handler(args)
            return {"result": result}
        except Exception as e:
            return {"error": str(e)}

    def _handle_init(self, args: Dict[str, Any]) -> str:
        project = args.get("project", self._project)
        self._store = FeatureStore.in_memory(project=project)
        self._project = project
        return f"Feature store initialized (project={project})"

    def _handle_apply_entity(self, args: Dict[str, Any]) -> str:
        name = args.get("name", "")
        join_keys = args.get("join_keys", [])
        self._store.apply_entity(name, join_keys)
        return f"Entity '{name}' applied"

    def _handle_apply_feature_view(self, args: Dict[str, Any]) -> str:
        name = args.get("name", "")
        entities = args.get("entities", [])
        features = args.get("features", [])
        ttl = args.get("ttl")
        self._store.apply_feature_view(name, entities, features, ttl)
        return f"FeatureView '{name}' applied"

    def _handle_list_entities(self, args: Dict[str, Any]) -> List[Dict[str, Any]]:
        return self._store.list_entities()

    def _handle_list_feature_views(self, args: Dict[str, Any]) -> List[Dict[str, Any]]:
        return self._store.list_feature_views()

    def _handle_materialize(self, args: Dict[str, Any]) -> str:
        start = args.get("start_date", 0)
        end = args.get("end_date", 0)
        self._store.materialize(start, end)
        return f"Materialized from {start} to {end}"

    def _handle_materialize_incremental(self, args: Dict[str, Any]) -> str:
        end = args.get("end_date", 0)
        self._store.materialize_incremental(end)
        return f"Incremental materialization up to {end}"

    def _handle_online_read(self, args: Dict[str, Any]) -> Any:
        entity_keys = args.get("entity_keys", {})
        features = args.get("features", [])
        entity_keys_bytes = {k: v.encode() if isinstance(v, str) else v for k, v in entity_keys.items()}
        result = self._store.online_read(entity_keys_bytes, features)
        return [r.decode() if r else None for r in result]

    def _handle_online_write(self, args: Dict[str, Any]) -> str:
        entity_keys = args.get("entity_keys", {})
        values = args.get("values", {})
        values_bytes = {k: v.encode() if isinstance(v, str) else v for k, v in values.items()}
        entity_keys_bytes = {k: v.encode() if isinstance(v, str) else v for k, v in entity_keys.items()}
        self._store.online_write(entity_keys_bytes, values_bytes)
        return "Values written"


def main() -> None:
    """Run the MCP server. Reads JSON requests from stdin, writes JSON responses to stdout."""
    store = MCPFeatureStore()
    print("OpenFeatureStore MCP server ready", file=sys.stderr)

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
            response = store.handle_request(request)
            print(json.dumps(response), flush=True)
        except json.JSONDecodeError:
            print(json.dumps({"error": "Invalid JSON"}), flush=True)
        except Exception as e:
            print(json.dumps({"error": str(e)}), flush=True)


if __name__ == "__main__":
    main()
