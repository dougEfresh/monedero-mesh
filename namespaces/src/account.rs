use crate::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Accounts(pub BTreeSet<Account>);

impl Deref for Accounts {
    type Target = BTreeSet<Account>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, SerializeDisplay, DeserializeFromStr,
)]
pub struct Account {
    pub address: String,
    pub chain: ChainId,
}

impl FromStr for Account {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let chain = ChainId::from_str(s)?;
        let components: Vec<&str> = s.split(':').collect();
        if components.len() != 3 {
            return Err(crate::Error::InvalidAccountFormat(String::from(s)));
        }

        Ok(Self {
            address: String::from(components[2]),
            chain,
        })
    }
}

impl Accounts {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for Account {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.chain, self.address)
    }
}
