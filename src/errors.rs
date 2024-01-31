use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("Resource not found")]
    ResourceNotFound,

    #[error("The provided path contains invalid characters")]
    InvalidPath,

    #[error("Failed to decode file contents")]
    DecodeError,

    #[error("network error")]
    Network(),

    #[error(transparent)]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, thiserror::Error)]
pub enum MirrorError {
    #[error("Mirror name not found")]
    MirrorFailedOnStores(BTreeMap<String, DriverError>),

    #[error("Mirror failed on store")]
    MirrorFailedOnStore(String, DriverError),
}

pub type DriverResult<T> = std::result::Result<T, DriverError>;
pub type MirrorResult<T> = std::result::Result<T, MirrorError>;
