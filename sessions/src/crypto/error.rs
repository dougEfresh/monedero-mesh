use walletconnect_sdk::rpc::domain::{ClientIdDecodingError, Topic};

#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("Unknown topic {0:#?}. Was the session closed/deleted?")]
    UnknownTopic(Topic),

    #[error("Unknown topic {0:#?}")]
    UnknownSessionTopic(Topic),

    #[error("Encryption error")]
    EncryptionError,

    #[error("Corrupted payload")]
    CorruptedPayload,

    #[error("chacha20poly1305 error")]
    Corrupted,

    #[error(transparent)]
    CorruptedString(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    DecodeError(#[from] data_encoding::DecodeError),

    #[error(transparent)]
    StorageError(#[from] crate::storage::Error),

    #[error(transparent)]
    CorruptedPacket(#[from] serde_json::error::Error),

    #[error(transparent)]
    ClientIdDecodingError(#[from] ClientIdDecodingError),

    #[error("Invalid key length")]
    InvalidKeyLength,

    #[error("failed to get lock on cipher store")]
    LockError,

    #[error("No Pairing exits")]
    NonExistingPairing,
}
