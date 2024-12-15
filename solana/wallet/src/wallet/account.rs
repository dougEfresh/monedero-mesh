use super::WALLET_FEATURES;
use monedero_signer_solana::SolanaSession;
use solana_pubkey::Pubkey;
use std::fmt::{Debug, Display};
use wallet_standard::prelude::*;

#[derive(Clone)]
pub struct ReownAccountInfo {
    pk: Pubkey,
}

impl ReownAccountInfo {
    pub fn new_session(session: &SolanaSession) -> Self {
        Self {
            pk: session.pubkey(),
        }
    }
    fn fmt_common(&self) -> String {
        format!("{}", self.pk)
    }
}

impl Display for ReownAccountInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}

impl Debug for ReownAccountInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}

impl WalletAccountInfo for ReownAccountInfo {
    fn address(&self) -> String {
        self.pk.to_string()
    }

    fn public_key(&self) -> Vec<u8> {
        self.pk.to_bytes().to_vec()
    }

    fn chains(&self) -> Vec<String> {
        vec!["solana".into()]
    }

    fn features(&self) -> Vec<String> {
        WALLET_FEATURES.map(Into::into).to_vec()
    }

    fn label(&self) -> Option<String> {
        None
    }

    fn icon(&self) -> Option<String> {
        None
    }
}
