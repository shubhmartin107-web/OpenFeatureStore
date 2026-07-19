# ofs-backfill

Backfill engine for materializing historical feature data.

- **BackfillEngine** — chunked, parallel materialization with configurable parallelism
- **Checkpoint/Resume** — persists progress to the registry for resumable backfills
- **Progress Tracking** — tracks rows processed, errors, and elapsed time per job
- **Cancel Support** — in-flight jobs can be cancelled via the registry
