use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signature;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid solana public key {0:#?}")]
    InvalidAccount(String),

    #[error("current wallet-connect does not have solana namespace")]
    NoSolanaNamespace,

    #[error("current wallet-connect does not have solana accounts")]
    SolanaAccountNotFound,

    #[error(transparent)]
    InvalidPubkey(#[from] solana_sdk::pubkey::ParsePubkeyError),

    #[error(transparent)]
    BincodeEncodeError(#[from] bincode::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::error::Error),

    #[error(transparent)]
    RpcError(#[from] monedero_mesh::Error),

    #[error("error decoding bs58: #{0}")]
    Bs58Error(String),

    #[error("invalid signature from wallet-connect: {0:#?}")]
    InvalidSignature(crate::SolanaSignatureResponse),

    #[error("failed to load keypair from bytes")]
    KeyPairFailure,

    #[error(transparent)]
    Base64Error(#[from] base64::DecodeError),

    #[error(transparent)]
    SignerError(#[from] solana_sdk::signer::SignerError),

    #[error(transparent)]
    SolanaRpcError(#[from] solana_rpc_client_api::client_error::Error),

    #[error(transparent)]
    SolanaProgramError(#[from] solana_program::program_error::ProgramError),

    #[error(transparent)]
    TransactionError(#[from] solana_sdk::transaction::TransactionError),

    #[error(transparent)]
    TokenError(#[from] spl_token_client::token::TokenError),

    #[error("signature failed to confirm {0}")]
    ConfirmationFailure(Signature),

    #[cfg(feature = "mock")]
    #[error("got a transaction but I have nothing to sign")]
    NothingToSign,

    #[error("spl-token program is not valid for this operation try spl-token-2022")]
    InvalidTokenProgram,

}
