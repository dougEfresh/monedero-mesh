use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::NamespaceName;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Events(pub BTreeSet<Event>);

impl Default for Events {
    fn default() -> Self {
        Self(BTreeSet::from([
            Event::AccountsChanged,
            Event::ChainChanged,
        ]))
    }
}

impl Deref for Events {
    type Target = BTreeSet<Event>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

///               "chainChanged",
//               "accountsChanged",
//               "disconnect",
//               "connect",
//               "message"
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, SerializeDisplay, DeserializeFromStr,
)]
pub enum Event {
    AccountsChanged,
    ChainChanged,
    Other(String),
}

impl From<&NamespaceName> for Events {
    fn from(value: &NamespaceName) -> Self {
        match value {
            NamespaceName::EIP155 => Self(BTreeSet::from([
                Event::AccountsChanged,
                Event::ChainChanged,
            ])),
            _ => Self(BTreeSet::new()),
        }
    }
}

impl FromStr for Event {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "chainchanged" => Ok(Self::ChainChanged),
            "accountschanged" => Ok(Self::AccountsChanged),
            _ => Ok(Self::Other(String::from(s))),
        }
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccountsChanged => write!(f, "accountsChanged"),
            Self::ChainChanged => write!(f, "chainChanged"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::AccountsChanged
    }
}
