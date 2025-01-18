mod error;
mod reown;
mod wallet;
pub use monedero_signer_solana::domain;
pub use monedero_signer_solana::session;
pub use {
    error::*,
    reown::*,
    wallet::*,
    wasm_client_solana::{SolanaRpcClient as RpcClient, DEVNET, MAINNET},
};
pub type Result<T> = std::result::Result<T, Error>;
