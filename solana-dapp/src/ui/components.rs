mod history;
mod nav;
pub mod popups;

use crate::message::{Msg, UserEvent};
use tui_realm_stdlib::{List, Phantom, Textarea};
use tuirealm::{
    command::{
        Cmd,
        CmdResult::{self},
    },
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, TableBuilder, TextSpan},
    AttrValue, Attribute, Component, Event, MockComponent, Sub,
};

use crate::ui::Id;
pub use history::*;
use monedero_solana::monedero_mesh::Pairing;
pub use nav::*;

pub trait ComponentMeta: Component<Msg, UserEvent> {
    fn id() -> Id;
    fn subscriptions() -> Vec<Sub<Id, UserEvent>>;
}

#[derive(Default, MockComponent)]
pub struct GlobalListener {
    component: Phantom,
}

impl Component<Msg, UserEvent> for GlobalListener {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => Some(Msg::ShowQuitPopup),
            /*
            Event::Keyboard(KeyEvent {
                              code: Key::Char('r'),
                              modifiers: KeyModifiers::CONTROL,
                            }) => Some(Msg::FetchAllSources),
            Event::Keyboard(KeyEvent {
                              code: Key::Char('r'),
                              ..
                            }) => Some(Msg::FetchSource),
             */
            _ => None,
        }
    }
}

#[derive(MockComponent)]
pub struct ShortcutsLegend {
    component: List,
}

impl Default for ShortcutsLegend {
    fn default() -> Self {
        Self {
            component: List::default()
                .title("Key Bindings", Alignment::Left)
                .scroll(false)
                .borders(Borders::default().modifiers(BorderType::Double))
                .rows(
                    TableBuilder::default()
                        .add_col(TextSpan::from(" ESC").bold())
                        .add_col(TextSpan::from("  "))
                        .add_col(TextSpan::from("Quit the application"))
                        .add_col(TextSpan::from("           "))
                        .add_col(TextSpan::from(" A").bold())
                        .add_col(TextSpan::from("  "))
                        .add_col(TextSpan::from("Add note/item"))
                        .add_row()
                        .add_col(TextSpan::from(" TAB").bold())
                        .add_col(TextSpan::from("  "))
                        .add_col(TextSpan::from("Switch focus"))
                        .add_col(TextSpan::from("                   "))
                        .add_col(TextSpan::from(" E").bold())
                        .add_col(TextSpan::from("  "))
                        .add_col(TextSpan::from("Edit note/item"))
                        .add_row()
                        .add_col(TextSpan::from(" SPC").bold())
                        .add_col(TextSpan::from("  "))
                        .add_col(TextSpan::from("Cycle between item status"))
                        .add_col(TextSpan::from("      "))
                        .add_col(TextSpan::from(" D").bold())
                        .add_col(TextSpan::from("  "))
                        .add_col(TextSpan::from("Delete note/item"))
                        .build(),
                ),
        }
    }
}

impl Component<Msg, UserEvent> for ShortcutsLegend {
    fn on(&mut self, _ev: Event<UserEvent>) -> Option<Msg> {
        None
    }
}
