pub mod external;
pub mod internal;

use serde::{Deserialize, Serialize};
pub use {external::ExternalMsg, internal::InternalMsg};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum MsgIn {
    Internal(internal::InternalMsg),
    External(external::ExternalMsg),
}
