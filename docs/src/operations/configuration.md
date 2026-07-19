# Configuration

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `DUCKDB_LIB_DIR` | Yes (build) | Path to directory containing `libduckdb.so` |
| `LD_LIBRARY_PATH` | Yes (runtime) | Must include `DUCKDB_LIB_DIR` |
| `OFS_PROJECT` | No | Default project name |
| `OFS_ONLINE_STORE` | No | Online store type (`sqlite`, `redis`) |
| `OFS_REDIS_URL` | No | Redis connection string |
| `RUST_LOG` | No | Log level (e.g., `info`, `debug`) |

## YAML Configuration

The server is configured via a YAML file (`ofs.yaml`, `ofs.yml`, or `~/.config/openfeaturestore/config.yaml`).

### Server

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  grpc_port: 8081
  max_request_size_mb: 10
  rate_limit:
    enabled: true
    default_rps: 100
  tls:
    enabled: false
    cert_path: /path/to/cert.pem
    key_path: /path/to/key.pem
```

> **Note:** Native TLS termination is not yet implemented. Use a reverse proxy (nginx, Caddy, Envoy, or a cloud load balancer) for TLS in production.

### Authentication

```yaml
auth:
  provider: noop          # "noop", "api_key", or "jwt"
  api_keys:
    - key: "sk-my-key"    # inline key
      key_env: "OFS_ADMIN_KEY"  # or reference an env var
      role: "admin"        # "admin", "write", or "read"
  jwt:
    jwks_url: "https://auth.example.com/.well-known/jwks.json"
    audience: "my-service"
    issuer: "https://auth.example.com/"
```

Auth providers:

| Provider | Description |
|---|---|
| `noop` | No authentication (default). All requests pass through. |
| `api_key` | Authenticate via `X-Api-Key` header. Supports multiple keys with roles. |
| `jwt` | Authenticate via `Bearer` token. Validates RS256 JWTs against a JWKS endpoint. Supports `realm_access.roles` and `resource_access.<client>.roles` for RBAC. |

### RBAC Roles

**Global roles** (applied to all projects):

| Role | Permissions |
|---|---|
| `admin` | Read, Write, Admin |
| `write` | Read, Write |
| `read` | Read |

**Project-level roles** (from JWT `realm_access.roles` or `resource_access.<client>.roles`):

| Role | Permissions |
|---|---|
| `admin` | Read, Write, Admin |
| `writer` | Read, Write |
| `reader` | Read |

Health (`/health`) and metrics (`/metrics`) endpoints are always exempt from authentication.

### Streaming

```yaml
stream_ingest:
  push_enabled: true
  push_buffer_size: 1000
  kafka:
    enabled: false
    brokers: "localhost:9092"
    group_id: "feature-store"
    topics: ["feature-events"]
    auto_offset_reset: "earliest"
```

### Caching

```yaml
cache:
  l1:
    max_size: 10000
    ttl_seconds: 300
  l2:
    enabled: false
    url: "redis://localhost:6379"
    key_prefix: "ofs:cache:"
    ttl_seconds: 900
  warming:
    enabled: false
    interval_seconds: 60
```

## RepoConfig

The `RepoConfig` struct controls store behavior:

```rust
pub struct RepoConfig {
    pub project: String,
    pub online_store: String,
    pub offline_store: String,
    pub registry_type: String,
    pub cache_ttl_seconds: i64,
    pub redis_host: Option<String>,
    pub redis_port: Option<u16>,
    pub redis_db: Option<i64>,
}
```

## DuckDB Configuration

DuckDB is linked at build time using `DUCKDB_LIB_DIR`:

```bash
export DUCKDB_LIB_DIR=/tmp/duckdb-lib
export LD_LIBRARY_PATH=/tmp/duckdb-lib:$LD_LIBRARY_PATH
cargo build
```

The DuckDB library is searched in:
1. `DUCKDB_LIB_DIR` environment variable
2. System library paths
