use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub request_id: String,
    pub action: String,
    pub project: String,
    pub entity_key: Option<String>,
    pub user: Option<String>,
    pub feature_names: Vec<String>,
    pub result: String,
    pub duration_ms: u64,
}

impl AuditEntry {
    pub fn new(
        request_id: impl Into<String>,
        action: impl Into<String>,
        project: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            request_id: request_id.into(),
            action: action.into(),
            project: project.into(),
            entity_key: None,
            user: None,
            feature_names: Vec::new(),
            result: "success".into(),
            duration_ms: 0,
        }
    }

    pub fn with_entity_key(mut self, key: impl Into<String>) -> Self {
        self.entity_key = Some(key.into());
        self
    }

    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.feature_names = features;
        self
    }

    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = result.into();
        self
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }
}

pub struct AuditLogger {
    path: Option<PathBuf>,
    buffer: Mutex<Vec<AuditEntry>>,
}

impl AuditLogger {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            buffer: Mutex::new(Vec::new()),
        }
    }

    pub fn log(&self, entry: AuditEntry) {
        if let Ok(mut buffer) = self.buffer.lock()
            && self.path.is_some()
        {
            buffer.push(entry);
            if buffer.len() >= 100 {
                let entries: Vec<AuditEntry> = buffer.drain(..).collect();
                if let Err(e) = self.flush_inner(&entries) {
                    tracing::error!("failed to flush audit log: {e}");
                }
            }
        }
    }

    pub fn flush(&self) {
        if let Ok(mut buffer) = self.buffer.lock() {
            let entries: Vec<AuditEntry> = buffer.drain(..).collect();
            if !entries.is_empty() {
                self.flush_entries(&entries);
            }
        }
    }

    fn flush_entries(&self, entries: &[AuditEntry]) {
        if let Err(e) = self.flush_inner(entries) {
            tracing::error!("failed to flush audit log: {e}");
        }
    }

    fn flush_inner(&self, entries: &[AuditEntry]) -> std::io::Result<()> {
        if let Some(ref path) = self.path {
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;

            for entry in entries {
                let line = serde_json::to_string(entry).unwrap_or_default();
                writeln!(file, "{line}")?;
            }
        }
        Ok(())
    }
}

impl Drop for AuditLogger {
    fn drop(&mut self) {
        self.flush();
    }
}

unsafe impl Send for AuditLogger {}
unsafe impl Sync for AuditLogger {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_audit_entry_builder() {
        let entry = AuditEntry::new("req-1", "get_online_features", "my_project")
            .with_entity_key("user:42")
            .with_user("alice")
            .with_features(vec!["feat1".into(), "feat2".into()])
            .with_result("success")
            .with_duration(15);

        assert_eq!(entry.request_id, "req-1");
        assert_eq!(entry.entity_key, Some("user:42".into()));
        assert_eq!(entry.feature_names.len(), 2);
    }

    #[test]
    fn test_audit_logger_file() {
        let dir = std::env::temp_dir().join("ofs-audit-test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("audit.log");
        let logger = AuditLogger::new(Some(path.clone()));

        let entry = AuditEntry::new("req-1", "get_online_features", "test");
        logger.log(entry);
        logger.flush();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("req-1"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_audit_logger_noop() {
        let logger = AuditLogger::new(None);
        let entry = AuditEntry::new("req-1", "get_online_features", "test");
        logger.log(entry);
    }
}
