use console::Term;
use monedero_solana::{SolanaSession, SolanaWallet};

pub struct Context {
    pub sol_session: SolanaSession,
    pub wallet: SolanaWallet,
    pub term: Term,
}

impl Context {}
