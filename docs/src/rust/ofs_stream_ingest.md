# ofs-stream-ingest

Streaming ingestion for real-time feature data.

- **WriteAheadLog** — SQLite-backed deduplication for exactly-once semantics
- **DeadLetterQueue** — stores failed records for replay and cleanup
- **PushIngestEngine** — processes HTTP push records through WAL → online store → DLQ
- **KafkaIngestEngine** — Kafka consumer with WAL dedup and DLQ fallback (behind `kafka` feature)
