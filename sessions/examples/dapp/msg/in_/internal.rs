use serde::{Deserialize, Serialize};

use crate::input::Key;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum InternalMsg {
    AddLastFocus(String, Option<String>),
    HandleKey(Key),
    RefreshSelection,
}
