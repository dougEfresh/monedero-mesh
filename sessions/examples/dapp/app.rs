use crate::config::Mode;
use crate::input::Key;
pub use crate::msg::in_::ExternalMsg;
pub use crate::msg::in_::InternalMsg;
pub use crate::msg::in_::MsgIn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub msg: MsgIn,
    pub key: Option<Key>,
}

impl Task {
    pub fn new(msg: MsgIn, key: Option<Key>) -> Self {
        Self { msg, key }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct App {
    pub mode: Mode,
    pub layout: crate::ui::AppLayout,
    //pub input: InputBuffer,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Default::default(),
            layout: Default::default(),
        }
    }
}
