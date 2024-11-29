use {
    crate::input::Key,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum InternalMsg {
    AddLastFocus(String, Option<String>),
    HandleKey(Key),
    RefreshSelection,
}
