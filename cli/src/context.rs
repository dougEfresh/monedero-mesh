use console::Term;
use monedero_solana::monedero_mesh::Dapp;
use monedero_solana::{
    ReownSigner, SolanaSession, SolanaWallet, TokenAccountsClient, TokenTransferClient,
};

pub struct Confirmation(String);

impl Confirmation {
    pub fn proceed(&self) -> bool {
        self.0.contains("y")
    }
}

impl From<String> for Confirmation {
    fn from(value: String) -> Self {
        Self(value)
    }
}

pub struct Context {
    pub sol_session: SolanaSession,
    pub wallet: SolanaWallet,
    pub term: Term,
}
