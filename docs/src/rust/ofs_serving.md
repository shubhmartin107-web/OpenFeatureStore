# ofs-serving

REST (axum) and gRPC (tonic) serving layer for OpenFeatureStore.

## FeatureServer

The main server type that binds REST and gRPC listeners. Configured via `OpenFeatureStoreConfig`.

### REST Endpoints

| Method | Path | Description |
|---|---|---|
| GET | `/health` | Health check |
| POST | `/v1/features:get-online` | Get online features |
| POST | `/v1/features:write-online` | Write online features |
| POST | `/v1/features:push` | Push feature data |

### Middleware Stack

1. **Auth** — authenticates and authorizes requests (skips `/health`, `/metrics`)
2. **Tracing** — request ID and span per request
3. **Compression** — gzip response compression
4. **CORS** — permissive CORS for development
5. **Rate Limiting** — token-bucket rate limiter (configurable)

TLS termination is expected to be handled by a reverse proxy.
