mod eip;
mod solana;

pub use crate::method::eip::EipMethod;
pub use crate::method::solana::SolanaMethod;
use crate::name::NamespaceName;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Methods(pub BTreeSet<Method>);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, SerializeDisplay, DeserializeFromStr,
)]
pub enum Method {
    EIP155(EipMethod),
    Solana(SolanaMethod),
    Other(String),
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EIP155(m) => write!(f, "{m}"),
            Self::Solana(m) => write!(f, "{m}"),
            Self::Other(m) => write!(f, "{m}"),
        }
    }
}

impl FromStr for Method {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let m = EipMethod::from_str(s)?;
        if !matches!(m, EipMethod::Other(_)) {
            return Ok(Self::EIP155(m));
        }

        let m = SolanaMethod::from_str(s)?;
        if !matches!(m, SolanaMethod::Other(_)) {
            return Ok(Self::Solana(m));
        }

        tracing::debug!("unclassified method: {s}. Please add to Methods enum");
        Ok(Self::Other(String::from(s)))
    }
}

impl Deref for Methods {
    type Target = BTreeSet<Method>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&NamespaceName> for Methods {
    fn from(value: &NamespaceName) -> Self {
        match value {
            NamespaceName::EIP155 => Self(EipMethod::defaults().into_iter().collect()),
            NamespaceName::Solana => Self(SolanaMethod::defaults().into_iter().collect()),
            NamespaceName::Other(_) => Self(BTreeSet::new()),
        }
    }
}

impl From<NamespaceName> for Methods {
    fn from(value: NamespaceName) -> Self {
        Self::from(&value)
    }
}
