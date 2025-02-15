use {
    crate::Topic,
    reown_relay_rpc::rpc::{PublishError, SubscriptionError},
};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Disconnected")]
    Disconnected,

    #[error("Got request to publish from a client who does not exists {0}")]
    InvalidConnectionState(Topic),

    #[error("client is not subscribed to {0}")]
    NotSubscribed(Topic),

    #[error("failed to broadcast event")]
    TxSendError,

    #[error(transparent)]
    NetworkError(#[from] reown_relay_client::error::ClientError),

    #[error(transparent)]
    SubscriptionError(#[from] reown_relay_client::error::Error<SubscriptionError>),

    #[error("failed to generate jwt key")]
    JwtError,

    #[error(transparent)]
    PublicationError(#[from] reown_relay_client::error::Error<PublishError>),

    #[error(transparent)]
    BindError(#[from] tokio::io::Error),
}
