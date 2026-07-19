# ofs-cache

Multi-tier caching for online feature reads.

- **FeatureCache trait** — common interface for cache implementations
- **L1Cache** — in-memory cache using moka (TinyLFU eviction, configurable max_size/TTL)
- **L2Cache** — Redis-backed cache with key prefix support and SCAN-based clear
- **CachedOnlineStore** — decorator implementing `OnlineStore` with L1→L2→inner store read path
- **CacheWarmer** — background task for periodic pre-population with graceful shutdown
