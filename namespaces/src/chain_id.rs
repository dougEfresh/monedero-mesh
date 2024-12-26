use {
    serde::{Deserialize, Serialize},
    serde_with::{DeserializeFromStr, SerializeDisplay},
    std::{
        cmp::Ordering,
        collections::BTreeSet,
        fmt::{self, Display, Formatter},
        ops::Deref,
        str::FromStr,
    },
};

// old solana:4sGjMW1sUnHzSxGspuhpqLDx6wiyjNtZ
// const SOLANA_NEW: &str = "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp";
// const SOLANA_OLD: &str = "solana:4sGjMW1sUnHzSxGspuhpqLDx6wiyjNtZ";
const SOLANA: &str = "solana:4sGjMW1sUnHzSxGspuhpqLDx6wiyjNtZ";
/// This is actually Solana Dev
// const SOLANA_DEV_NEW: &str = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1";
const SOLANA_DEV: &str = "solana:8E9rvCKLFQia2Y35HXjjpWzj8weVo44K";
const SOLANA_TEST: &str = "solana:testnet";

#[derive(
    Debug,
    Copy,
    Default,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Ord,
    PartialOrd,
    SerializeDisplay,
    DeserializeFromStr,
)]
pub enum ChainType {
    Main,
    Test,
    #[default]
    Dev,
}

impl FromStr for ChainType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "main" | "mainnet" => Ok(Self::Main),
            "test" | "testnet" => Ok(Self::Test),
            _ => Ok(Self::Dev),
        }
    }
}

impl Display for ChainType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Main => "main",
                Self::Test => "testnet",
                Self::Dev => "devnet",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub enum ChainId {
    EIP155(alloy_chains::Chain),
    Solana(ChainType),
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Chains(pub BTreeSet<ChainId>);

impl<const N: usize> From<[ChainId; N]> for Chains {
    fn from(value: [ChainId; N]) -> Self {
        Self(BTreeSet::from_iter(value))
    }
}

impl FromIterator<ChainId> for Chains {
    fn from_iter<T: IntoIterator<Item = ChainId>>(iter: T) -> Self {
        Self(BTreeSet::from_iter(iter))
    }
}

impl Default for Chains {
    fn default() -> Self {
        Self(BTreeSet::from([ChainId::Solana(ChainType::Dev)]))
    }
}

impl Display for Chains {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();
        if let Some(first) = iter.next() {
            write!(f, "{first}")?;
            for chain in iter {
                write!(f, ", {chain}")?;
            }
        }
        Ok(())
    }
}

impl IntoIterator for Chains {
    type IntoIter = std::collections::btree_set::IntoIter<ChainId>;
    type Item = ChainId;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// Implement IntoIterator for &Chains
impl<'a> IntoIterator for &'a Chains {
    type IntoIter = std::collections::btree_set::Iter<'a, ChainId>;
    type Item = &'a ChainId;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// Implement IntoIterator for &mut Chains
#[allow(clippy::into_iter_without_iter)]
impl<'a> IntoIterator for &'a mut Chains {
    type IntoIter = std::collections::btree_set::Iter<'a, ChainId>;
    type Item = &'a ChainId;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Deref for Chains {
    type Target = BTreeSet<ChainId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialOrd for ChainId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChainId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl Default for ChainId {
    fn default() -> Self {
        Self::Solana(ChainType::Main)
    }
}

impl FromStr for ChainId {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let components = s.split(':').collect::<Vec<_>>();
        // Can be either in format:
        // {ns}:{chainId}:{account}
        // {ns}:{chainId}
        if components.is_empty() || components.len() == 1 {
            return Err(crate::Error::MalformedChainId(String::from(s)));
        }
        let ns = components[0].to_lowercase();
        let id = components[1].to_string();
        if ns.starts_with("eip155") {
            if let Ok(id) = id.parse::<u64>() {
                return Ok(Self::EIP155(alloy_chains::Chain::from(id)));
            }
            return Err(crate::Error::InvalidChainId(String::from(s)));
        }
        let chain_id = format!("{ns}:{id}");
        match chain_id.as_str() {
            SOLANA => Ok(Self::Solana(ChainType::Main)),
            SOLANA_DEV => Ok(Self::Solana(ChainType::Dev)),
            SOLANA_TEST => Ok(Self::Solana(ChainType::Test)),
            _ => {
                tracing::debug!("unknown chain {}", s);
                Ok(Self::Other(s.to_string()))
            }
        }
    }
}

impl Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EIP155(chain) => write!(f, "eip155:{}", chain.id()),
            Self::Solana(ChainType::Main) => write!(f, "{SOLANA}"),
            Self::Solana(ChainType::Dev) => write!(f, "{SOLANA_DEV}"),
            Self::Solana(ChainType::Test) => write!(f, "{SOLANA_TEST}"),
            // ChainId::Near(ChainType::Main) => write!(f, "mainnet"),
            // ChainId::Near(ChainType::Test) => write!(f, "testnet"),
            // ChainId::Tezos(ChainType::Main) => write!(f, "mainnet"),
            // ChainId::Tezos(ChainType::Test) => write!(f, "testnet"),
            // ChainId::Cosmos(ChainType::Main) => write!(f, "mainnet"),
            // ChainId::Cosmos(ChainType::Test) => write!(f, "testnet"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl From<alloy_chains::Chain> for ChainId {
    fn from(value: alloy_chains::Chain) -> Self {
        Self::EIP155(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_id() -> anyhow::Result<()> {
        let eip155 = ChainId::from_str("eip155:1")?;
        assert!(matches!(eip155, ChainId::EIP155(_)));
        assert_eq!(eip155.to_string(), "eip155:1");

        let solana = ChainId::from_str(SOLANA)?;
        assert!(matches!(solana, ChainId::Solana(_)));
        assert_eq!(solana.to_string(), SOLANA);
        assert_eq!(solana, SOLANA.parse()?);

        let with_account = format!("{SOLANA}:someaccount");
        let solana = ChainId::from_str(&with_account)?;
        assert!(matches!(solana, ChainId::Solana(_)));

        let solana = ChainId::from_str(SOLANA_DEV)?;
        assert!(matches!(solana, ChainId::Solana(ChainType::Dev)));
        assert_eq!(solana.to_string(), SOLANA_DEV);
        assert_eq!(solana, SOLANA_DEV.parse()?);

        // let solana = ChainId::from_str(SOLANA_TEST_OLD)?;
        // assert!(matches!(solana, ChainId::Solana(ChainType::Test)));
        // assert_eq!(solana.to_string(), SOLANA_TEST_OLD);
        // assert_eq!(solana, SOLANA_TEST_OLD.parse()?);
        Ok(())
    }
}
