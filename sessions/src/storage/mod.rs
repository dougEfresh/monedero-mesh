#[cfg(not(feature = "wasm"))]
mod os;
#[cfg(not(feature = "wasm"))]
pub use os::KvStorage;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    StorageInit(#[from] microxdg::XdgError),

    #[error(transparent)]
    StorageErr(#[from] kvx::Error),

    #[error("failed to parse key ${0:}")]
    SegmentErr(String),

    #[error(transparent)]
    LocationUnknown(#[from] url::ParseError),

    #[error("{key} is not found")]
    NotFound { key: String },

    #[error("failed to lock storage")]
    LockFailed,

    #[error("storage init error")]
    NamespaceInvalid,

    #[error(transparent)]
    MalformedJson(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, Error>;
