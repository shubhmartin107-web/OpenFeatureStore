use thiserror::Error;

#[derive(Error, Debug)]
pub enum OfsError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("DuckDB error: {0}")]
    DuckDb(String),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("materialization error: {0}")]
    Materialization(String),

    #[error("backfill error: {0}")]
    Backfill(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("authentication error: {0}")]
    Auth(String),

    #[error("authorization error: {0}")]
    Forbidden(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("invalid state transition: from {from:?} to {to:?}")]
    InvalidStateTransition { from: String, to: String },

    #[error("feature view {0} has no TTL and no prior materialization")]
    NoTtlAndNoMaterialization(String),

    #[error("protobuf decode error: {0}")]
    ProtoDecode(#[from] prost::DecodeError),

    #[error("protobuf encode error: {0}")]
    ProtoEncode(#[from] prost::EncodeError),

    #[error("chrono parse error: {0}")]
    ChronoParse(#[from] chrono::ParseError),

    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),
}

impl From<String> for OfsError {
    fn from(s: String) -> Self {
        OfsError::Internal(s)
    }
}

impl From<&str> for OfsError {
    fn from(s: &str) -> Self {
        OfsError::Internal(s.to_string())
    }
}

impl From<serde_json::Error> for OfsError {
    fn from(e: serde_json::Error) -> Self {
        OfsError::Serialization(e.to_string())
    }
}

pub type OfsResult<T> = Result<T, OfsError>;
