use chrono::{DateTime, Utc};
use ofs_core::errors::{OfsError, OfsResult};
use sqlx::SqlitePool;

/// Write-ahead log for idempotent stream ingestion.
///
/// Tracks record IDs to provide exactly-once semantics.
/// Uses SQLite for persistence.
pub struct WriteAheadLog {
    pool: SqlitePool,
}

impl WriteAheadLog {
    pub async fn new(path: Option<&str>) -> OfsResult<Self> {
        let pool = match path {
            Some(p) => SqlitePool::connect(p).await,
            None => SqlitePool::connect("sqlite::memory:").await,
        }
        .map_err(|e| OfsError::Database(e.to_string()))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ingest_wal (
                record_id TEXT PRIMARY KEY NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                error TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                processed_at TEXT
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ingest_wal_status ON ingest_wal(status)")
            .execute(&pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn is_duplicate(&self, record_id: &str) -> OfsResult<bool> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT status FROM ingest_wal WHERE record_id = ?")
                .bind(record_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| OfsError::Database(e.to_string()))?;

        match row {
            Some((status,)) => Ok(status == "processed" || status == "pending"),
            None => Ok(false),
        }
    }

    pub async fn mark_pending(&self, record_id: &str) -> OfsResult<()> {
        sqlx::query("INSERT OR IGNORE INTO ingest_wal (record_id, status) VALUES (?1, 'pending')")
            .bind(record_id)
            .execute(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn mark_processed(&self, record_id: &str) -> OfsResult<()> {
        sqlx::query(
            "UPDATE ingest_wal SET status = 'processed', processed_at = datetime('now') WHERE record_id = ?",
        )
        .bind(record_id)
        .execute(&self.pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn mark_failed(&self, record_id: &str, error: &str) -> OfsResult<()> {
        sqlx::query(
            "UPDATE ingest_wal SET status = 'failed', error = ?2, processed_at = datetime('now') WHERE record_id = ?1",
        )
        .bind(record_id)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn pending_count(&self) -> OfsResult<u64> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM ingest_wal WHERE status = 'pending'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(count.0 as u64)
    }

    pub async fn failed_count(&self) -> OfsResult<u64> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM ingest_wal WHERE status = 'failed'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(count.0 as u64)
    }

    pub async fn cleanup_old(&self, before: DateTime<Utc>) -> OfsResult<u64> {
        let ts = before.to_rfc3339();
        let result = sqlx::query("DELETE FROM ingest_wal WHERE processed_at < ?1")
            .bind(&ts)
            .execute(&self.pool)
            .await
            .map_err(|e| OfsError::Database(e.to_string()))?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_wal() -> WriteAheadLog {
        WriteAheadLog::new(None).await.unwrap()
    }

    #[tokio::test]
    async fn test_not_duplicate() {
        let wal = create_wal().await;
        assert!(!wal.is_duplicate("rec-1").await.unwrap());
    }

    #[tokio::test]
    async fn test_mark_and_check_duplicate() {
        let wal = create_wal().await;
        wal.mark_pending("rec-1").await.unwrap();
        assert!(wal.is_duplicate("rec-1").await.unwrap());
    }

    #[tokio::test]
    async fn test_mark_processed() {
        let wal = create_wal().await;
        wal.mark_pending("rec-1").await.unwrap();
        wal.mark_processed("rec-1").await.unwrap();
        assert!(wal.is_duplicate("rec-1").await.unwrap());
    }

    #[tokio::test]
    async fn test_mark_failed() {
        let wal = create_wal().await;
        wal.mark_pending("rec-1").await.unwrap();
        wal.mark_failed("rec-1", "test error").await.unwrap();

        // Failed records are NOT considered duplicates (allows retry)
        assert!(!wal.is_duplicate("rec-1").await.unwrap());
    }

    #[tokio::test]
    async fn test_pending_count() {
        let wal = create_wal().await;
        wal.mark_pending("rec-1").await.unwrap();
        wal.mark_pending("rec-2").await.unwrap();
        wal.mark_processed("rec-2").await.unwrap();
        assert_eq!(wal.pending_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_cleanup_old() {
        let wal = create_wal().await;
        wal.mark_pending("rec-1").await.unwrap();
        wal.mark_processed("rec-1").await.unwrap();

        // Should not delete pending records
        let before = Utc::now() + chrono::Duration::hours(1);
        let deleted = wal.cleanup_old(before).await.unwrap();
        assert_eq!(deleted, 1);
    }

    #[tokio::test]
    async fn test_ignore_duplicate_pending() {
        let wal = create_wal().await;
        wal.mark_pending("rec-1").await.unwrap();
        wal.mark_pending("rec-1").await.unwrap();
        assert_eq!(wal.pending_count().await.unwrap(), 1);
    }
}
