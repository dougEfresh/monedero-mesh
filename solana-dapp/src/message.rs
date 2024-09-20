use crate::DappContext;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};

#[derive(Debug, PartialEq, Clone, PartialOrd, Eq)]
pub enum UserEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    FocusGained,
    FocusLost,
    SettledError(String),
    Settled(DappContext),
}

impl From<char> for UserEvent {
    fn from(c: char) -> Self {
        UserEvent::Key(KeyEvent::from(KeyCode::Char(c)))
    }
}

impl From<KeyCode> for UserEvent {
    fn from(code: KeyCode) -> Self {
        UserEvent::Key(KeyEvent::from(code))
    }
}

impl From<KeyEvent> for UserEvent {
    fn from(ev: KeyEvent) -> Self {
        UserEvent::Key(ev)
    }
}

impl From<UserEvent> for Msg {
    fn from(value: UserEvent) -> Self {
        Self::User(value)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Msg {
    AppClose,
    None,
    ShowQuitPopup,
    Quit,
    CloseQuitPopup,
    CloseErrorPopup,
    ClosePairQrCode,
}

#[derive(PartialEq, Eq, Clone, PartialOrd)]
pub enum AppEvent {
    ErrorInitialized,
}

#[macro_export]
macro_rules! panic_set_hook {
    ($t:tt) => {
        use std::panic;
        let default_hook = panic::take_hook();

        panic::set_hook(Box::new(move |info| {
            $t;

            default_hook(info);
        }));
    };
}
