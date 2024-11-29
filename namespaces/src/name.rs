use {
    crate::chain_id::{ChainId, Chains},
    serde::{Deserialize, Serialize},
    serde_with::{DeserializeFromStr, SerializeDisplay},
    std::{
        cmp::Ordering,
        collections::BTreeSet,
        fmt::{self, Display},
        str::FromStr,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, SerializeDisplay, DeserializeFromStr)]
pub enum NamespaceName {
    EIP155,
    Solana,
    // Tezos,
    // Near,
    Other(String),
}

impl Default for NamespaceName {
    fn default() -> Self {
        Self::Solana
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NamespaceNames(pub BTreeSet<NamespaceName>);

impl PartialOrd for NamespaceName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NamespaceName {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl Display for NamespaceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EIP155 => write!(f, "eip155"),
            Self::Solana => write!(f, "solana"),
            // NamespaceName::Tezos => write!(f, "tezos"),
            // NamespaceName::Near => write!(f, "near"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl FromStr for NamespaceName {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "solana" => Ok(Self::Solana),
            "eip155" => Ok(Self::EIP155),
            _ => Ok(Self::Other(String::from(s))),
        }
    }
}

impl From<&str> for NamespaceName {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "solana" => Self::Solana,
            "eip155" => Self::EIP155,
            _ => Self::Other(String::from(value)),
        }
    }
}

impl From<&ChainId> for NamespaceName {
    fn from(value: &ChainId) -> Self {
        match value {
            ChainId::EIP155(_) => Self::EIP155,
            ChainId::Solana(_) => Self::Solana,
            ChainId::Other(arg) => {
                let mut it = arg.split(':');
                if let Some(ns) = it.next() {
                    return Self::Other(String::from(ns));
                }
                Self::Other(String::from(arg))
            }
        }
    }
}

impl From<&Chains> for NamespaceNames {
    fn from(value: &Chains) -> Self {
        Self(value.into_iter().map(NamespaceName::from).collect())
    }
}

impl From<Chains> for NamespaceNames {
    fn from(value: Chains) -> Self {
        Self(value.into_iter().map(NamespaceName::from).collect())
    }
}

impl From<ChainId> for NamespaceName {
    fn from(value: ChainId) -> Self {
        Self::from(&value)
    }
}

impl From<NamespaceName> for String {
    fn from(name: NamespaceName) -> Self {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_name() {
        assert_eq!(NamespaceName::EIP155.to_string(), "eip155");
        assert_eq!(NamespaceName::Solana.to_string(), "solana");
        assert_eq!(
            NamespaceName::Other(String::from("blah")).to_string(),
            "blah"
        );

        let s: NamespaceName = "solana".into();
        assert_eq!(NamespaceName::Solana, s);
    }
}
