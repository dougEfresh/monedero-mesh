#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid solana public key {0:#?}")]
    InvalidAccount(String),

    #[error("current wallet-connect does not have solana namespace")]
    NoSolanaNamespace,

    #[error("current wallet-connect does not have solana accounts")]
    NoSolanaAccounts,

    #[error(transparent)]
    InvalidPubkey(#[from] solana_sdk::pubkey::ParsePubkeyError),
}
