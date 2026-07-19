use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::error::RemoteResult;

#[derive(Debug, Clone)]
struct CacheEntry {
    local_path: PathBuf,
    size: u64,
    last_accessed: DateTime<Utc>,
}

#[derive(Clone)]
pub struct RemoteCache {
    cache_dir: PathBuf,
    max_size_bytes: u64,
    entries: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

impl RemoteCache {
    pub fn new(cache_dir: Option<PathBuf>, max_size_mb: u64) -> Self {
        let cache_dir = cache_dir.unwrap_or_else(|| std::env::temp_dir().join("ofs-remote-cache"));
        Self {
            cache_dir,
            max_size_bytes: max_size_mb * 1024 * 1024,
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub async fn init(&self) -> RemoteResult<()> {
        tokio::fs::create_dir_all(&self.cache_dir).await?;
        info!(
            cache_dir = %self.cache_dir.display(),
            max_size_mb = %(self.max_size_bytes / (1024 * 1024)),
            "initialized remote file cache"
        );
        Ok(())
    }

    pub async fn get(&self, remote_path: &str) -> RemoteResult<Option<PathBuf>> {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(remote_path) {
            if tokio::fs::try_exists(&entry.local_path)
                .await
                .unwrap_or(false)
            {
                entry.last_accessed = Utc::now();
                debug!(remote_path, cached_path = %entry.local_path.display(), "cache hit");
                return Ok(Some(entry.local_path.clone()));
            }
            entries.remove(remote_path);
        }
        Ok(None)
    }

    pub async fn insert(
        &self,
        remote_path: &str,
        local_path: PathBuf,
        size: u64,
    ) -> RemoteResult<PathBuf> {
        let mut entries = self.entries.lock().await;
        entries.insert(
            remote_path.to_string(),
            CacheEntry {
                local_path: local_path.clone(),
                size,
                last_accessed: Utc::now(),
            },
        );

        let total: u64 = entries.values().map(|e| e.size).sum();
        if total > self.max_size_bytes {
            self.evict_lru(&mut entries).await;
        }

        Ok(local_path)
    }

    async fn evict_lru(&self, entries: &mut HashMap<String, CacheEntry>) {
        let mut sorted: Vec<(String, CacheEntry)> = entries.drain().collect();
        sorted.sort_by_key(|(_, e)| e.last_accessed);

        let mut total: u64 = sorted.iter().map(|(_, e)| e.size).sum();
        let target = self.max_size_bytes / 2;

        let mut keep = Vec::new();
        for (key, entry) in sorted {
            if total > target {
                total = total.saturating_sub(entry.size);
                drop(tokio::fs::remove_file(&entry.local_path));
                debug!(path = %entry.local_path.display(), "evicted from cache");
            } else {
                keep.push((key, entry));
            }
        }

        for (key, entry) in keep {
            entries.insert(key, entry);
        }
    }

    pub async fn clear(&self) -> RemoteResult<()> {
        let mut entries = self.entries.lock().await;
        for (_, entry) in entries.drain() {
            drop(tokio::fs::remove_file(&entry.local_path));
        }
        info!("cleared remote file cache");
        Ok(())
    }

    pub fn temp_path_for(&self, remote_path: &str) -> PathBuf {
        let file_name = remote_path.split('/').next_back().unwrap_or("remote_file");
        self.cache_dir.join(format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            file_name
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = RemoteCache::new(None, 100);
        let result = cache.get("s3://bucket/file.parquet").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_insert_and_get() {
        let dir = std::env::temp_dir().join("ofs-cache-test");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let test_file = dir.join("test.parquet");
        tokio::fs::write(&test_file, b"test data").await.unwrap();

        let cache = RemoteCache::new(Some(dir.clone()), 100);
        let result = cache
            .insert("s3://bucket/file.parquet", test_file.clone(), 9)
            .await
            .unwrap();
        assert_eq!(result, test_file);

        let cached = cache.get("s3://bucket/file.parquet").await.unwrap();
        assert_eq!(cached, Some(test_file));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let dir = std::env::temp_dir().join("ofs-cache-clear-test");
        let _ = tokio::fs::create_dir_all(&dir).await;
        let cache = RemoteCache::new(Some(dir.clone()), 100);

        let test_file = dir.join("test_file.parquet");
        tokio::fs::write(&test_file, b"data").await.unwrap();

        cache
            .insert("test.parquet", test_file.clone(), 4)
            .await
            .unwrap();

        // verify it's cached
        let cached = cache.get("test.parquet").await.unwrap();
        assert!(cached.is_some());

        cache.clear().await.unwrap();
        let result = cache.get("test.parquet").await.unwrap();
        assert!(result.is_none());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
