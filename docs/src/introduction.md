# OpenFeatureStore

**OpenFeatureStore** is a high-performance, Feast-compatible offline and online feature store
built in Rust. It provides point-in-time correct feature retrieval via ASOF joins, SQLite-backed
registry and online storage, DuckDB-powered offline analytics, and a materialization engine for
serving feature data from offline to online stores.

## Why OpenFeatureStore?

- **Performance** — Written in Rust with zero-cost abstractions and async I/O
- **Feast Compatible** — Protobuf definitions match Feast 0.42+ schema
- **Point-in-Time Correctness** — ASOF joins ensure features are joined at the right time
- **Offline/Online Architecture** — Batch compute on DuckDB, low-latency serving from SQLite or Redis
- **Materialization Engine** — Bridges offline and online stores with incremental updates
- **Python SDK** — Native Python bindings via PyO3
- **CLI & MCP** — Command-line tool and MCP server for opencode integration

## Architecture Overview

```
                 ┌──────────────────────┐
                 │   Python SDK / CLI   │
                 │   MCP Server         │
                 └──────────┬───────────┘
                            │
              ┌─────────────┴─────────────┐
              │    ofs-core (Traits)      │
              │  Registry | OfflineStore  │
              │  OnlineStore | Materialize│
              └──────┬──────────┬─────────┘
                     │          │
          ┌──────────┴──┐  ┌───┴────────────┐
          │ ofs-registry │  │ ofs-offline    │
          │  (SQLite)    │  │  (DuckDB)      │
          └──────────────┘  └────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    │ ofs-online              │
                    │  (SQLite | Redis)       │
                    └─────────────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    │ ofs-materialization     │
                    │  (Offline → Online)     │
                    └─────────────────────────┘
```

## Quick Links

- [Quick Start](quick_start.md) — Get up and running in 5 minutes
- [Installation](installation.md) — Build and install instructions
- [GitHub Repository](https://github.com/anomalyco/openfeaturestore)
- [Crate Documentation](https://docs.rs/openfeaturestore)
