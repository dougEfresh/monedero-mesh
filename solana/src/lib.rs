mod error;

pub use error::Error;
use solana_sdk::pubkey::Pubkey;
use std::ops::Deref;
use std::str::FromStr;
use walletconnect_namespaces::NamespaceName;
use walletconnect_sessions::ClientSession;

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct Solana {
    pk: Pubkey,
    session: ClientSession,
}

impl TryFrom<&ClientSession> for Solana {
    type Error = Error;

    fn try_from(value: &ClientSession) -> std::result::Result<Self, Self::Error> {
        let ns = value
            .namespaces()
            .get(&NamespaceName::Solana)
            .ok_or(Error::NoSolanaNamespace)?;
        let account = ns.accounts.deref().first().ok_or(Error::NoSolanaAccounts)?;
        Ok(Self {
            pk: Pubkey::from_str(&account.address)?,
            session: value.clone(),
        })
    }
}
