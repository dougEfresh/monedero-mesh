use {console::Term, copypasta::ClipboardContext, monedero_solana::SolanaWallet};

pub struct Context {
    pub wallet: SolanaWallet,
    pub term: Term,
    pub clip: ClipboardContext,
}

impl Context {}
