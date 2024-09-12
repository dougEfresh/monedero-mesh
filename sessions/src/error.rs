use crate::Topic;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to receive the proposed value")]
    ReceiveError,

    #[error("client is not initialized")]
    NoClient,

    #[error("failed to get mutex lock")]
    LockError,

    #[error("failed to store Pairing")]
    PairingInitError,

    #[error(transparent)]
    ActorSendError(#[from] xtra::Error),

    #[error(transparent)]
    ConnectError(#[from] monedero_relay::ClientError),

    #[error(transparent)]
    CorruptedPacket(#[from] serde_json::error::Error),

    #[error("no session account")]
    NoSessionAccount,

    #[error("failed to generate jwt key")]
    JwtError,

    #[error(transparent)]
    CipherError(#[from] crate::crypto::CipherError),

    #[error(transparent)]
    StorageError(#[from] crate::storage::Error),

    #[error("Timeout waiting for session settlement")]
    SessionSettlementTimeout,

    #[error("Failed to recv response from request id: {0}")]
    ResponseChannelError(crate::domain::MessageId),

    #[error("Timeout waiting for session request")]
    SessionRequestTimeout,

    #[error("Timeout waiting for response")]
    ResponseTimeout,

    #[error("recv channel closed for settlement request")]
    SettlementRecvError,

    #[error("Settlement was rejected by the wallet provider: '{0:#?}'")]
    SettlementRejected(String),

    #[error("a party has rejected the settlement")]
    ProposalRejected,

    #[error("Got session settlement but I have no one to send this to!")]
    SessionSettlementNotFound,

    #[error("RPC error {0:#?}")]
    RpcError(serde_json::Value),

    #[error("No pairing topic available")]
    NoPairingTopic,

    #[error("No pending handler for settlement on pairing topic {0:#?}")]
    InvalidPendingHandler(Topic),

    #[error(transparent)]
    ParamsError(#[from] crate::rpc::ParamsError),

    #[error("This error goes back to the origninal request")]
    RpcErrorFromRequest(crate::rpc::RpcErrorResponse),

    #[error(transparent)]
    PairingParseError(#[from] crate::pairing_uri::ParseError),

    #[cfg(feature = "mock")]
    #[error("Must supply ConnectionsOptions when mock feature is used")]
    InvalidateConnectionOpts,

    #[error("No wallet found to handle request on topic {0:#?}")]
    NoWalletHandler(Topic),

    #[error("No pairing manager for {0:#?}")]
    NoPairManager(Topic),

    #[error("No client session for {0:#?}")]
    NoClientSession(Topic),
}
