# CLI Commands

## `init`

Initialize a new feature store.

```bash
ofs init [--project PROJECT]
```

## `apply-entity`

Register an entity.

```bash
ofs apply-entity <name> [--join-keys KEYS] [--project PROJECT]
```

**Arguments:**
- `name` — Entity name (required)

**Options:**
- `--join-keys` — Comma-separated list of join key names

**Example:**
```bash
ofs apply-entity user --join-keys user_id
ofs apply-entity transaction --join-keys transaction_id,user_id
```

## `apply-feature-view`

Register a feature view.

```bash
ofs apply-feature-view <name> [options]
```

**Arguments:**
- `name` — Feature view name (required)

**Options:**
- `--entities` — Comma-separated entity names
- `--features` — Comma-separated feature names
- `--ttl` — TTL in seconds

**Example:**
```bash
ofs apply-feature-view user_features \
    --entities user \
    --features age,gender,signup_date \
    --ttl 86400
```

## `list-entities`

List all registered entities.

```bash
ofs list-entities [--project PROJECT]
```

## `list-feature-views`

List all registered feature views.

```bash
ofs list-feature-views [--project PROJECT]
```

## `materialize`

Materialize features from offline to online store.

```bash
ofs materialize --start-date UNIX_TS --end-date UNIX_TS [--project PROJECT]
```

**Options:**
- `--start-date` — Start Unix timestamp (required)
- `--end-date` — End Unix timestamp (required)

## `incremental`

Incremental materialization.

```bash
ofs incremental --end-date UNIX_TS [--project PROJECT]
```

**Options:**
- `--end-date` — End Unix timestamp (required)
