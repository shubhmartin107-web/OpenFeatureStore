use thiserror::Error;

#[derive(Error, Debug)]
pub enum RemoteStoreError {
    #[error("unsupported URI scheme: {0}")]
    UnsupportedScheme(String),

    #[error("missing credential: {0}")]
    MissingCredential(String),

    #[error("object store error: {0}")]
    ObjectStore(#[from] object_store::Error),

    #[error("URI parse error: {0}")]
    UriParse(#[from] url::ParseError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("path not found: {0}")]
    NotFound(String),

    #[error("invalid field: {0}")]
    InvalidField(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),
}

pub type RemoteResult<T> = Result<T, RemoteStoreError>;
