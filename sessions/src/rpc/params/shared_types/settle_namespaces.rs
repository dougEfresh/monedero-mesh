use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettleNamespace {
    pub accounts: BTreeSet<String>,
    pub methods: BTreeSet<String>,
    pub events: BTreeSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub extensions: Option<Vec<Self>>,
}
