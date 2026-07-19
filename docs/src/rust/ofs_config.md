# ofs-config

YAML-based configuration for OpenFeatureStore. Supports environment variable interpolation (`${VAR}`) and secret resolution from environment (`OFS_SECRET_*`).

## Key Types

- **OpenFeatureStoreConfig** — top-level config with `server`, `auth`, `registry`, `online_store`, `offline_store`, `cache`, `stream_ingest` sections
- **ServerConfig** — host, port, gRPC port, rate limiting, TLS settings
- **AuthConfig** — provider selection (`noop`, `api_key`, `jwt`), API key entries, JWT config
- **CacheConfig** — L1 (moka) and L2 (Redis) cache settings
- **StreamIngestConfig** — push endpoint and optional Kafka consumer settings
