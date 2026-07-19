# Monitoring

## Logging

The feature store uses the `log` crate. Set log level via `RUST_LOG`:

```bash
RUST_LOG=info ofs materialize --start-date 1700000000 --end-date 1700086400
RUST_LOG=debug ofs list-entities
```

## Metrics

Key metrics to monitor:

| Metric | Description |
|---|---|
| `materialization_duration_ms` | Time to materialize a feature view |
| `online_read_latency_ms` | Online read p50/p99 latency |
| `online_write_throughput` | Batch write throughput |
| `offline_query_duration_ms` | Offline query execution time |
| `registry_operation_latency_ms` | Registry CRUD latency |

## Health Checks

Store backends expose health check methods:

```rust
// Registry
let healthy = registry.health_check().await?;

// Online store
let healthy = online_store.health_check().await?;

// Offline store
let healthy = offline_store.health_check().await?;
```

## Alerting

Recommended alerts:

- **Stale materialization**: No materialization in >24 hours
- **Online store errors**: >1% error rate on reads
- **Offline query failures**: Failed queries > threshold
- **High latency**: Online reads >100ms p99
