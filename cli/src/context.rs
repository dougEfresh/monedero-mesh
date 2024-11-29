use {console::Term, monedero_solana::SolanaWallet};

pub struct Context {
    pub wallet: SolanaWallet,
    pub term: Term,
}

impl Context {}
