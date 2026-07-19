use std::env;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, ConfigResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigSource {
    pub paths: Vec<PathBuf>,
    pub env_prefix: String,
    pub secrets_from_env: bool,
}

impl Default for ConfigSource {
    fn default() -> Self {
        Self {
            paths: vec![
                PathBuf::from("./ofs.yaml"),
                PathBuf::from("./ofs.yml"),
                dirs::config_dir()
                    .map(|p| p.join("openfeaturestore").join("config.yaml"))
                    .unwrap_or_default(),
            ]
            .into_iter()
            .filter(|p| !p.as_os_str().is_empty())
            .collect(),
            env_prefix: String::from("OFS_"),
            secrets_from_env: true,
        }
    }
}

impl ConfigSource {
    pub fn from_path<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            paths: vec![path.into()],
            ..Default::default()
        }
    }

    pub fn resolve_path(&self) -> ConfigResult<PathBuf> {
        for p in &self.paths {
            let expanded = shellexpand::tilde(&p.to_string_lossy()).to_string();
            let resolved = PathBuf::from(&expanded);
            if resolved.exists() {
                return Ok(resolved);
            }
        }
        Err(ConfigError::FileNotFound(format!(
            "no config file found at {:?}",
            self.paths
        )))
    }
}

pub fn resolve_env_var(key: &str) -> Option<String> {
    let unprefixed = key.trim_start_matches("${").trim_end_matches('}');
    env::var(unprefixed).ok()
}

pub fn interpolate_env_vars(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next();
            let mut var_name = String::new();
            while let Some(&next) = chars.peek() {
                if next == '}' {
                    chars.next();
                    break;
                }
                var_name.push(next);
                chars.next();
            }
            let resolved = env::var(&var_name).unwrap_or_else(|_| {
                tracing::warn!("env var {} not set, leaving placeholder", var_name);
                format!("${{{}}}", var_name)
            });
            result.push_str(&resolved);
        } else {
            result.push(c);
        }
    }

    result
}

pub fn resolve_secret(key: &str) -> Option<String> {
    let env_key = format!("OFS_SECRET_{}", key.to_uppercase().replace('-', "_"));
    env::var(&env_key).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_simple() {
        // SAFETY: test-only env var mutation, single-threaded
        unsafe { env::set_var("OFS_TEST_VAR", "hello") };
        assert_eq!(interpolate_env_vars("${OFS_TEST_VAR}"), "hello");
        unsafe { env::remove_var("OFS_TEST_VAR") };
    }

    #[test]
    fn test_interpolate_with_surrounding() {
        unsafe { env::set_var("OFS_DB_PATH", "/data/db") };
        assert_eq!(
            interpolate_env_vars("sqlite://${OFS_DB_PATH}/features.db"),
            "sqlite:///data/db/features.db"
        );
        unsafe { env::remove_var("OFS_DB_PATH") };
    }

    #[test]
    fn test_interpolate_unset_var() {
        let result = interpolate_env_vars("${UNSET_VAR}");
        assert_eq!(result, "${UNSET_VAR}");
    }

    #[test]
    fn test_resolve_secret() {
        unsafe { env::set_var("OFS_SECRET_API_KEY", "sk-1234") };
        assert_eq!(resolve_secret("api-key"), Some("sk-1234".into()));
        unsafe { env::remove_var("OFS_SECRET_API_KEY") };
    }

    #[test]
    fn test_resolve_secret_not_found() {
        assert_eq!(resolve_secret("nonexistent"), None);
    }
}
