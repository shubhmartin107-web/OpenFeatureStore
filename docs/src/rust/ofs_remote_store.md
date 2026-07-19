# ofs-remote-store

Cloud storage abstraction for offline feature data.

- **RemoteBackend** — S3, GCS, and Azure Blob Storage via the `object_store` crate
- **RemoteCache** — LRU eviction cache for remote files, stages data locally for DuckDB
- **URI parsing** — automatic backend selection from URI scheme (`s3://`, `gs://`, `az://`)
