//! See [solana](https://docs.walletconnect.com/advanced/multichain/rpc-reference/solana-rpc) methods

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::method::Method;

const SIGN_MESSAGE: &str = "solana_signMessage";
const SIGN_TRANSACTION: &str = "solana_signTransaction";

#[derive(Debug, Clone, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub enum SolanaMethod {
    SignMessage,
    SignTransaction,
    Other(String),
}

impl Ord for SolanaMethod {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl PartialOrd for SolanaMethod {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for SolanaMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SignMessage => write!(f, "{SIGN_MESSAGE}"),
            Self::SignTransaction => write!(f, "{SIGN_TRANSACTION}"),
            Self::Other(m) => write!(f, "{m}"),
        }
    }
}

impl FromStr for SolanaMethod {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            SIGN_TRANSACTION => Ok(Self::SignTransaction),
            SIGN_MESSAGE => Ok(Self::SignMessage),
            _ => Ok(Self::Other(s.to_string())),
        }
    }
}

impl SolanaMethod {
    #[must_use]
    pub fn defaults() -> BTreeSet<Method> {
        BTreeSet::from([
            Method::Solana(Self::SignTransaction),
            Method::Solana(Self::SignMessage),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_method() -> anyhow::Result<()> {
        assert_eq!(
            SolanaMethod::SignTransaction,
            SIGN_TRANSACTION.parse::<SolanaMethod>()?
        );
        assert_eq!(
            SolanaMethod::SignMessage,
            SIGN_MESSAGE.parse::<SolanaMethod>()?
        );
        assert!(matches!(
            "solana_signAndSend".parse::<SolanaMethod>()?,
            SolanaMethod::Other(_)
        ));
        Ok(())
    }
}
