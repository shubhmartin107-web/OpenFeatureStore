use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    FileNotFound(String),

    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse YAML: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("missing required field: {0}")]
    MissingField(String),

    #[error("invalid field value: {0}")]
    InvalidField(String),

    #[error("env var {0} not set and no default provided")]
    MissingEnvVar(String),

    #[error("failed to resolve path: {0}")]
    PathResolution(String),
}

pub type ConfigResult<T> = Result<T, ConfigError>;
