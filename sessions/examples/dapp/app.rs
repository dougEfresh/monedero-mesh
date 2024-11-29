use {
    crate::{config::Mode, input::Key},
    anyhow::Result,
    serde::{Deserialize, Serialize},
    std::collections::VecDeque,
};

pub use crate::msg::{
    in_::{ExternalMsg, InternalMsg, MsgIn},
    out::MsgOut,
};

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
    // pub input: InputBuffer,
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
        let app = match task.msg {
            MsgIn::Internal(msg) => self.handle_internal(msg)?,
            MsgIn::External(msg) => self.handle_external(msg, task.key)?,
        };
        app.refresh()
    }

    fn refresh(mut self) -> Result<Self> {
        self.msg_out.push_back(MsgOut::Refresh);
        Ok(self)
    }

    fn handle_internal(self, msg: InternalMsg) -> Result<Self> {
        match msg {
            InternalMsg::AddLastFocus(parent, focus_path) => {
                // self.add_last_focus(parent, focus_path)
                Ok(self)
            }
            InternalMsg::HandleKey(key) => self.handle_key(key),
            InternalMsg::RefreshSelection => {
                // self.refresh_selection(),
                Ok(self)
            }
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    fn handle_key(mut self, key: Key) -> Result<Self> {
        let kb = self.mode.key_bindings.clone();
        let key_str = key.to_string();
        let msgs = kb
            .on_key
            .get(&key_str)
            .map(|a| a.messages.clone())
            .or_else(|| {
                if key.is_alphabet() {
                    kb.on_alphabet.as_ref().map(|a| a.messages.clone())
                } else if key.is_number() {
                    kb.on_number.as_ref().map(|a| a.messages.clone())
                } else if key.is_special_character() {
                    kb.on_special_character.as_ref().map(|a| a.messages.clone())
                } else if key.is_navigation() {
                    kb.on_navigation.as_ref().map(|a| a.messages.clone())
                } else if key.is_function() {
                    kb.on_function.as_ref().map(|a| a.messages.clone())
                } else {
                    None
                }
            })
            .or_else(|| {
                if key.is_alphanumeric() {
                    kb.on_alphanumeric.as_ref().map(|a| a.messages.clone())
                } else {
                    None
                }
            })
            .or_else(|| {
                if key.is_character() {
                    kb.on_character.as_ref().map(|a| a.messages.clone())
                } else {
                    None
                }
            })
            .or_else(|| kb.default.as_ref().map(|a| a.messages.clone()))
            .unwrap_or_else(|| vec![ExternalMsg::LogWarning("key map not found.".into())]);

        for msg in msgs {
            // Rename breaks without enqueue
            let external = MsgIn::External(msg);
            let task = Task::new(external, Some(key));
            let msg_out = MsgOut::Enqueue(task);
            self.msg_out.push_back(msg_out);
        }

        Ok(self)
    }

    fn handle_external(mut self, msg: ExternalMsg, key: Option<Key>) -> Result<Self> {
        self = match msg {
            ExternalMsg::Quit => self.quit()?,
            _ => self,
        };
        Ok(self)
    }

    fn quit(mut self) -> Result<Self> {
        self.msg_out.push_back(MsgOut::Quit);
        Ok(self)
    }
}
