#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Namespace not found")]
    NamespaceNotFound,

    #[error("Method is invalid: {0:#?}")]
    InvalidMethod(String),
    #[error("Invalid account format {0:#?}")]
    InvalidAccountFormat(String),
    #[error("Invalid chain ID {0:#?}")]
    InvalidChainId(String),

    #[error("Invalid event {0:#?}")]
    InvalidEvent(String),

    #[error("chainId has incorrect syntax {0:#?}")]
    MalformedChainId(String),
}
