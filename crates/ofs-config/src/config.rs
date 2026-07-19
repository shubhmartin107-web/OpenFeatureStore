use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, ConfigResult};
use crate::source::{ConfigSource, interpolate_env_vars, resolve_secret};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ApiKeyEntry {
    pub key: Option<String>,
    pub key_env: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct JwtConfig {
    pub jwks_url: Option<String>,
    pub audience: Option<String>,
    pub issuer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AuthConfig {
    #[serde(default = "default_auth_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_keys: Vec<ApiKeyEntry>,
    pub jwt: Option<JwtConfig>,
}

fn default_auth_provider() -> String {
    "noop".into()
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            provider: "noop".into(),
            api_keys: Vec::new(),
            jwt: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RateLimitTier {
    pub name: String,
    pub rps: u32,
    #[serde(default = "default_burst")]
    pub burst: u32,
}

fn default_burst() -> u32 {
    10
}

impl Default for RateLimitTier {
    fn default() -> Self {
        Self {
            name: String::new(),
            rps: 100,
            burst: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RateLimitConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_default_rps")]
    pub default_rps: u32,
    #[serde(default)]
    pub tiers: Vec<RateLimitTier>,
}

fn default_true() -> bool {
    true
}

fn default_default_rps() -> u32 {
    100
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_rps: 100,
            tiers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    #[serde(default = "default_max_request_size_mb")]
    pub max_request_size_mb: u32,
    pub rate_limit: Option<RateLimitConfig>,
    pub tls: Option<TlsConfig>,
}

fn default_host() -> String {
    "0.0.0.0".into()
}

fn default_port() -> u16 {
    8080
}

fn default_grpc_port() -> u16 {
    8081
}

fn default_max_request_size_mb() -> u32 {
    10
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 8080,
            grpc_port: 8081,
            max_request_size_mb: 10,
            rate_limit: Some(RateLimitConfig::default()),
            tls: None,
        }
    }
}

/// TLS is terminated at the reverse proxy (nginx). This config is
/// informational only — it documents cert paths for the proxy setup.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", default)]
pub struct TlsConfig {
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct PostgresConfig {
    pub connection_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegistryConfig {
    #[serde(default = "default_registry_backend")]
    pub backend: String,
    #[serde(default = "default_registry_path")]
    pub path: String,
    pub postgres: Option<PostgresConfig>,
}

fn default_registry_backend() -> String {
    "sqlite".into()
}

fn default_registry_path() -> String {
    "./ofs.db".into()
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".into(),
            path: "./ofs.db".into(),
            postgres: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", default)]
pub struct OfflineStoreConfig {
    pub backend: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RedisConfig {
    pub connection_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", default)]
pub struct OnlineStoreConfig {
    pub backend: String,
    pub path: String,
    pub redis: Option<RedisConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct S3Config {
    pub region: Option<String>,
    pub bucket: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct GcsConfig {
    pub bucket: Option<String>,
    pub service_account_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct AzureConfig {
    pub container: Option<String>,
    pub connection_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", default)]
pub struct RemoteStoreConfig {
    pub enabled: bool,
    pub s3: Option<S3Config>,
    pub gcs: Option<GcsConfig>,
    pub azure: Option<AzureConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct KafkaConfig {
    pub brokers: Option<String>,
    pub topic: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HttpPushConfig {
    #[serde(default = "default_http_push_batch")]
    pub max_batch_size: usize,
}

fn default_http_push_batch() -> usize {
    100
}

impl Default for HttpPushConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StreamIngestConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_stream_source")]
    pub source: String,
    pub kafka: Option<KafkaConfig>,
    pub http_push: Option<HttpPushConfig>,
}

fn default_stream_source() -> String {
    "http_push".into()
}

impl Default for StreamIngestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            source: "http_push".into(),
            kafka: None,
            http_push: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LifecycleConfig {
    #[serde(default = "default_ttl_days")]
    pub ttl_default_days: u64,
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval_secs: u64,
    #[serde(default = "default_projects")]
    pub projects: Vec<String>,
}

fn default_ttl_days() -> u64 {
    90
}

fn default_cleanup_interval() -> u64 {
    3600
}

fn default_projects() -> Vec<String> {
    vec!["default".to_string()]
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            ttl_default_days: 90,
            cleanup_interval_secs: 3600,
            projects: vec!["default".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ObservabilityConfig {
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,
    #[serde(default = "default_true")]
    pub tracing_enabled: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_log_format")]
    pub log_format: String,
    pub audit_log_path: Option<String>,
}

fn default_log_level() -> String {
    "info".into()
}

fn default_log_format() -> String {
    "json".into()
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            tracing_enabled: true,
            log_level: "info".into(),
            log_format: "json".into(),
            audit_log_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RedisCacheConfig {
    pub nodes: Vec<String>,
    pub password: Option<String>,
    pub mode: Option<String>,
    pub key_prefix: Option<String>,
    pub default_ttl_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CacheConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_cache_max_size")]
    pub max_size: usize,
    #[serde(default = "default_cache_ttl_secs")]
    pub ttl_secs: u64,
    pub redis: Option<RedisCacheConfig>,
}

fn default_cache_max_size() -> usize {
    10_000
}

fn default_cache_ttl_secs() -> u64 {
    300
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size: 10_000,
            ttl_secs: 300,
            redis: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MaterializationConfig {
    #[serde(default = "default_materialization_parallelism")]
    pub parallelism: usize,
    #[serde(default = "default_materialization_batch_size")]
    pub batch_size: usize,
}

fn default_materialization_parallelism() -> usize {
    4
}

fn default_materialization_batch_size() -> usize {
    1000
}

impl Default for MaterializationConfig {
    fn default() -> Self {
        Self {
            parallelism: 4,
            batch_size: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OpenFeatureStoreConfig {
    #[serde(default = "default_config_version")]
    pub version: String,
    pub server: Option<ServerConfig>,
    pub tls: Option<TlsConfig>,
    pub auth: Option<AuthConfig>,
    pub registry: Option<RegistryConfig>,
    pub offline_store: Option<OfflineStoreConfig>,
    pub online_store: Option<OnlineStoreConfig>,
    pub remote_store: Option<RemoteStoreConfig>,
    pub stream_ingest: Option<StreamIngestConfig>,
    pub lifecycle: Option<LifecycleConfig>,
    pub observability: Option<ObservabilityConfig>,
    pub cache: Option<CacheConfig>,
    pub materialization: Option<MaterializationConfig>,
}

fn default_config_version() -> String {
    "1".into()
}

impl Default for OpenFeatureStoreConfig {
    fn default() -> Self {
        Self {
            version: "1".into(),
            server: Some(ServerConfig::default()),
            tls: Some(TlsConfig::default()),
            auth: Some(AuthConfig::default()),
            registry: Some(RegistryConfig::default()),
            offline_store: Some(OfflineStoreConfig {
                backend: "duckdb".into(),
                path: "./ofs-offline.db".into(),
            }),
            online_store: Some(OnlineStoreConfig {
                backend: "sqlite".into(),
                path: "./ofs-online.db".into(),
                redis: None,
            }),
            remote_store: Some(RemoteStoreConfig::default()),
            stream_ingest: Some(StreamIngestConfig::default()),
            lifecycle: Some(LifecycleConfig::default()),
            observability: Some(ObservabilityConfig::default()),
            cache: Some(CacheConfig::default()),
            materialization: Some(MaterializationConfig::default()),
        }
    }
}

impl OpenFeatureStoreConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        let interpolated = interpolate_env_vars(&contents);
        let mut config: Self =
            serde_yaml::from_str(&interpolated).map_err(ConfigError::YamlParse)?;
        config.resolve_secrets()?;
        Ok(config)
    }

    pub fn from_source(source: &ConfigSource) -> ConfigResult<Self> {
        let path = source.resolve_path()?;
        Self::from_file(path)
    }

    fn resolve_secrets(&mut self) -> ConfigResult<()> {
        if let Some(ref mut auth) = self.auth {
            for entry in &mut auth.api_keys {
                if let Some(ref env_key) = entry.key_env
                    && entry.key.is_none()
                {
                    entry.key = resolve_secret(env_key);
                }
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> ConfigResult<()> {
        if let Some(ref registry) = self.registry
            && !["sqlite", "postgres"].contains(&registry.backend.as_str())
        {
            return Err(ConfigError::InvalidField(format!(
                "unsupported registry backend: {}",
                registry.backend
            )));
        }
        if let Some(ref online) = self.online_store
            && !["sqlite", "redis"].contains(&online.backend.as_str())
        {
            return Err(ConfigError::InvalidField(format!(
                "unsupported online store backend: {}",
                online.backend
            )));
        }
        if let Some(ref offline) = self.offline_store
            && !["duckdb"].contains(&offline.backend.as_str())
        {
            return Err(ConfigError::InvalidField(format!(
                "unsupported offline store backend: {}",
                offline.backend
            )));
        }
        if let Some(ref server) = self.server
            && let Some(ref rate_limit) = server.rate_limit
        {
            for tier in &rate_limit.tiers {
                if tier.name.is_empty() {
                    return Err(ConfigError::InvalidField(
                        "rate limit tier name must not be empty".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OpenFeatureStoreConfig::default();
        assert_eq!(config.version, "1");
        assert_eq!(
            config.server.as_ref().map(|s| s.host.as_str()),
            Some("0.0.0.0")
        );
        assert_eq!(config.server.as_ref().map(|s| s.port), Some(8080));
        assert_eq!(config.server.as_ref().map(|s| s.grpc_port), Some(8081));
        assert_eq!(
            config.registry.as_ref().map(|r| r.backend.as_str()),
            Some("sqlite")
        );
        assert_eq!(
            config.offline_store.as_ref().map(|o| o.backend.as_str()),
            Some("duckdb")
        );
        assert_eq!(
            config.online_store.as_ref().map(|o| o.backend.as_str()),
            Some("sqlite")
        );
        assert_eq!(
            config.auth.as_ref().map(|a| a.provider.as_str()),
            Some("noop")
        );
        assert_eq!(
            config.observability.as_ref().map(|o| o.log_format.as_str()),
            Some("json")
        );
        assert_eq!(
            config.lifecycle.as_ref().map(|l| l.ttl_default_days),
            Some(90)
        );
        assert_eq!(
            config.materialization.as_ref().map(|m| m.parallelism),
            Some(4)
        );
    }

    #[test]
    fn test_validate_passes() {
        let config = OpenFeatureStoreConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_fails_bad_backend() {
        let mut config = OpenFeatureStoreConfig::default();
        config.registry = Some(RegistryConfig {
            backend: "mysql".into(),
            ..RegistryConfig::default()
        });
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_roundtrip_yaml() {
        let config = OpenFeatureStoreConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: OpenFeatureStoreConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(
            config.server.as_ref().map(|s| s.port),
            deserialized.server.as_ref().map(|s| s.port)
        );
        assert_eq!(
            config.registry.as_ref().map(|r| r.backend.as_str()),
            deserialized.registry.as_ref().map(|r| r.backend.as_str())
        );
    }

    #[test]
    fn test_from_file_not_found() {
        let result = OpenFeatureStoreConfig::from_file("/nonexistent/path.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_source_resolve_default() {
        let source = ConfigSource::default();
        assert!(!source.paths.is_empty());
    }

    #[test]
    fn test_deserialize_empty_yaml() {
        let yaml = "version: \"1\"\n";
        let config: OpenFeatureStoreConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.version, "1");
        assert!(config.server.is_none());
        assert!(config.registry.is_none());
    }

    #[test]
    fn test_deserialize_partial() {
        let yaml = r#"
version: "1"
server:
  host: "127.0.0.1"
  port: 9000
"#;
        let config: OpenFeatureStoreConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.version, "1");
        assert_eq!(
            config.server.as_ref().map(|s| s.host.as_str()),
            Some("127.0.0.1")
        );
        assert_eq!(config.server.as_ref().map(|s| s.port), Some(9000));
        assert!(config.registry.is_none());
    }
}
