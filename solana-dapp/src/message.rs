use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};

#[derive(Debug, PartialEq, Clone, PartialOrd, Eq)]
pub enum UserEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    FocusGained,
    FocusLost,
    Settled,
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

#[derive(Debug, PartialEq, Clone)]
pub enum Msg {
    AppClose,
    None,
    ShowQuitPopup,
    Quit,
    CloseQuitPopup,
    CloseErrorPopup,
    ClosePairQrCode,
    Settled,
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
