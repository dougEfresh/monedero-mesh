use crate::message::UserEvent;
use crate::Msg;
use tui_realm_stdlib::Table;
use tuirealm::props::Alignment;
use tuirealm::{Component, Event, MockComponent};

#[derive(MockComponent)]
pub struct Nav {
    component: Table,
}

impl Default for Nav {
    fn default() -> Self {
        Self {
            component: Table::default()
                .title("Nav", Alignment::Center)
                .headers(&["home", "transfer", "swap", "bridge"]),
        }
    }
}

impl Component<Msg, UserEvent> for Nav {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        None
    }
}
