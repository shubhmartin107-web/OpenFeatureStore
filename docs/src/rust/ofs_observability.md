# ofs-observability

Observability infrastructure for OpenFeatureStore.

- **OfsMetrics** — Prometheus counters/histograms for feature requests, store reads/writes
- **HealthRegistry** — composable health checks with overall status (Healthy/Degraded/Unhealthy)
- **AuditLogger** — append-only JSON-line audit logging with buffered writes
- **Tracing** — structured JSON logging via `tracing-subscriber` with env-filter support
