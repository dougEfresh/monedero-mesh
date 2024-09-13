use tui_realm_stdlib::{Input, List, Phantom, Textarea};
use tuirealm::{command::{
  Cmd,
  CmdResult::{self, Changed},
  Direction, Position,
}, event::{Key, KeyEvent, KeyModifiers}, props::{
  Alignment, BorderType, Borders, Color, InputType, Style, Table, TableBuilder, TextSpan,
}, AttrValue, Attribute, Component, Event, MockComponent, NoUserEvent};
use crate::message::{AppEvent, Msg};


#[derive(Default, MockComponent)]
pub struct GlobalListener {
  component: Phantom,
}

impl Component<Msg, AppEvent> for GlobalListener {
  fn on(&mut self, ev: Event<AppEvent>) -> Option<Msg> {
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
pub struct PairQrCode {
  component: Textarea
}

impl Default for PairQrCode {
  fn default() -> Self {
    Self {
      component: Textarea::default().title("pair", Alignment::Center).text_rows(&[TextSpan::new("no pairing established")]),
    }
  }
}

impl Component<Msg, AppEvent> for PairQrCode  {
  fn on(&mut self, ev: Event<AppEvent>) -> Option<Msg> {
    None
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

impl Component<Msg, AppEvent> for ShortcutsLegend {
  fn on(&mut self, _ev: Event<AppEvent>) -> Option<Msg> {
    None
  }
}
