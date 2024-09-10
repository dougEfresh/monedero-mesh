use crate::config::Mode;
use crate::input::Key;
pub use crate::msg::in_::ExternalMsg;
pub use crate::msg::in_::InternalMsg;
pub use crate::msg::in_::MsgIn;
use crate::msg::out::MsgOut;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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
    pub msg_out: VecDeque<MsgOut>,
    //pub input: InputBuffer,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::default(),
            layout: crate::ui::AppLayout::default(),
            msg_out: VecDeque::new(),
        }
    }

    pub fn handle_task(self, task: Task) -> Result<Self> {
        //   let app = match task.msg {
        //     MsgIn::Internal(msg) => self.handle_internal(msg)?,
        //   MsgIn::External(msg) => self.handle_external(msg, task.key)?,
        //};
        self.refresh()
    }

    fn refresh(mut self) -> Result<Self> {
        self.msg_out.push_back(MsgOut::Refresh);
        Ok(self)
    }
}
