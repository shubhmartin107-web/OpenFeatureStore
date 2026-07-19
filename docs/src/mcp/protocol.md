# MCP Protocol

## Request Format

Each request is a single JSON object on a line:

```json
{
  "tool": "<tool_name>",
  "args": { "<key>": "<value>", ... }
}
```

## Response Format

Success:

```json
{
  "success": true,
  "result": { ... }
}
```

Error:

```json
{
  "success": false,
  "error": "error message"
}
```

## Tool Details

### `init`

Initialize the feature store.

```json
{"tool": "init", "args": {"project": "demo"}}
```

Response: `{"success": true, "result": {"message": "FeatureStore initialized"}}`

### `apply_entity`

Register an entity.

```json
{
  "tool": "apply_entity",
  "args": {
    "name": "user",
    "join_keys": ["user_id"],
    "description": "A user",
    "owner": "data-team"
  }
}
```

### `apply_feature_view`

Register a feature view.

```json
{
  "tool": "apply_feature_view",
  "args": {
    "name": "user_features",
    "entities": ["user"],
    "features": ["age", "gender"],
    "ttl_secs": 86400
  }
}
```

### `list_entities`

```json
{"tool": "list_entities", "args": {}}
```

Response:
```json
{
  "success": true,
  "result": {
    "entities": [
      {"name": "user", "join_keys": ["user_id"], "description": "A user"}
    ]
  }
}
```

### `list_feature_views`

```json
{"tool": "list_feature_views", "args": {}}
```

### `materialize`

```json
{
  "tool": "materialize",
  "args": {
    "start_date": 1700000000.0,
    "end_date": 1700086400.0
  }
}
```

### `online_read`

```json
{
  "tool": "online_read",
  "args": {
    "entity_keys": {"user_id": "dXNlcl8wMDE="},
    "features": ["age", "gender"],
    "feature_view_name": "user_features"
  }
}
```

Entity key values must be base64-encoded byte strings.

### `online_write`

```json
{
  "tool": "online_write",
  "args": {
    "entity_keys": {"user_id": "dXNlcl8wMDE="},
    "values": {"age": "Mjg="},
    "feature_view_name": "user_features"
  }
}
```

All values must be base64-encoded byte strings.

## Error Handling

The server reads and processes one line at a time, writing the response to
stdout. Invalid JSON or missing required fields will result in an error response.
