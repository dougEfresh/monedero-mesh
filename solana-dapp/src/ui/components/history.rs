use crate::message::UserEvent;
use crate::Msg;
use tui_realm_stdlib::Textarea;
use tuirealm::props::{BorderType, Borders, TextSpan};
use tuirealm::{Component, Event, MockComponent};

#[derive(MockComponent)]
pub struct History {
    component: Textarea,
}

impl Default for History {
    fn default() -> Self {
        Self {
            component: Textarea::default()
                .borders(Borders::default().modifiers(BorderType::Double))
                .text_rows(&[TextSpan::new("history")]),
        }
    }
}

impl Component<Msg, UserEvent> for History {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::User(UserEvent::Settled) => {
                Some(Msg::Settled)
            }
            _ => None
        }
    }
}
