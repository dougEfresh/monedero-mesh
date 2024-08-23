use walletconnect_sdk::client::websocket::PublishedMessage;
use walletconnect_sdk::rpc::domain::{ClientIdDecodingError, MessageId};
use walletconnect_sdk::rpc::rpc::{PublishError, SubscriptionError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to decode payload from PublishedMessage {0:#?}")]
    DecodeError(PublishedMessage),

    #[error("client is not initialized")]
    NoClient,

    #[error("failed to get mutex lock")]
    LockError,

    #[error("failed to store Pairing")]
    PairingInitError,

    #[error(transparent)]
    ActorSendError(#[from] xtra::Error),

    #[error(transparent)]
    ConnectError(#[from] crate::relay::ClientError),

    #[error(transparent)]
    ClientIdDecodingError(#[from] ClientIdDecodingError),

    #[error(transparent)]
    CorruptedPacket(#[from] serde_json::error::Error),

    #[error("no session account")]
    NoSessionAccount,

    #[error(transparent)]
    SubscriptionError(#[from] walletconnect_sdk::client::error::Error<SubscriptionError>),

    #[error("failed to generate jwt key")]
    JwtError,

    #[error(transparent)]
    PublicationError(#[from] walletconnect_sdk::client::error::Error<PublishError>),

    #[error(transparent)]
    CipherError(#[from] crate::crypto::CipherError),

    #[error(transparent)]
    StorageError(#[from] crate::storage::Error),

    #[error("Timeout waiting for session settlement")]
    SessionSettlementTimeout,

    #[error("Failed to recv response from request id: {0}")]
    ResponseChannelError(MessageId),

    #[error("Timeout waiting for session request")]
    SessionRequestTimeout,

    #[error("recv channel closed for settlement request")]
    SettlementRecvError,

    #[error("Got session settlement but I have no one to send this to!")]
    SessionSettlementNotFound,

    #[error("RPC error {0:#?}")]
    RpcError(serde_json::Value),

    #[error("No pairing topic available")]
    NoPairingTopic,

    #[error(transparent)]
    ParamsError(#[from] crate::rpc::ParamsError),
}
