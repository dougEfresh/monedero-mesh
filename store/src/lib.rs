#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::KvStorage;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::KvStorage;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to init storage {0}")]
    StorageInit(String),

    #[cfg(target_arch = "wasm32")]
    #[error(transparent)]
    StorageErr(#[from] gloo_storage::errors::StorageError),

    #[cfg(not(target_arch = "wasm32"))]
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
