use std::time::Duration;
use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph},
    Frame,
};
use tuirealm::tui::layout::{Constraint, Direction, Layout, Rect};
use tuirealm::tui::widgets::Clear;
use tuirealm::{Application, EventListenerCfg, PollStrategy, Sub, SubClause, SubEventClause, Update};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::terminal::TerminalBridge;
use crate::app::App;
use crate::message::AppEvent;
use crate::Msg;
use crate::ui::components::*;
use crate::ui::popups::{ErrorPopup, QuitPopup};

mod components;
mod models;
mod popups;

/// Draw an area (WxH / 3) in the middle of the parent area
pub fn draw_area_in(parent: Rect, width: u16, height: u16) -> Rect {
    let new_area = Layout::default()
      .direction(Direction::Vertical)
      .constraints(
          [
              Constraint::Percentage((100 - height) / 2),
              Constraint::Percentage(height),
              Constraint::Percentage((100 - height) / 2),
          ]
            .as_ref(),
      )
      .split(parent);
    Layout::default()
      .direction(Direction::Horizontal)
      .constraints(
          [
              Constraint::Percentage((100 - width) / 2),
              Constraint::Percentage(width),
              Constraint::Percentage((100 - width) / 2),
          ]
            .as_ref(),
      )
      .split(new_area[1])[1]
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
enum Id {
    Pairing,
    Legend,
    GlobalListener,
    Home,
    ErrorPopup,
    QuitPopup,
}

pub struct Ui {
    application: Application<Id, Msg, AppEvent>,
    terminal: TerminalBridge,
    redraw: bool,
}

impl Ui {
    pub fn init() -> anyhow::Result<Self> {
        let ticks = Duration::from_millis(10);
        let mut app = Application::<Id, Msg, AppEvent>::init(
            EventListenerCfg::default()
              .default_input_listener(ticks)
              .poll_timeout(ticks),
        );
        app.mount(Id::Legend, Box::new(ShortcutsLegend::default()), vec![])?;
        app.mount(Id::GlobalListener, Box::new(GlobalListener::default()), Self::subs())?;
        app.active(&Id::Legend)?;
        let terminal =  TerminalBridge::new()?;
        Ok(Self { application: app , terminal, redraw: false })
    }


    /// initialize terminal
    pub(super) fn init_terminal(&mut self) {
        let _ = self.terminal.enable_raw_mode();
        let _ = self.terminal.enter_alternate_screen();
        let _ = self.terminal.clear_screen();
    }

    pub fn run(mut self)  {
        self.init_terminal();
        let mut quit = false;
        self.view();
        while !quit {
            // poll and update
            match self.application.tick(PollStrategy::UpTo(3)) {
                Ok(messages) if messages.is_empty() => {}
                Ok(messages) => {
                    self.redraw = true;
                    for msg in messages.into_iter() {
                        if let Some(Msg::Quit) = self.update(Some(msg)) {
                            quit = true;
                            break;
                        }
                    }
                }
                Err(err) => {
                    self.mount_error_popup(format!("Application error: {}", err));
                }
            }

            // View
            if self.redraw {
                self.view();
            }
        }
        self.finalize_terminal();
    }

    pub(super) fn view(&mut self) {
        let _ = self.terminal.raw_mut().draw(|f| {
            let main_chunks = Layout::default()
              .direction(Direction::Vertical)
              .margin(1)
              .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
              .split(f.size());

            self.application.view(&Id::Legend, f, main_chunks[0]);

            if self.application.mounted(&Id::QuitPopup) {
                let popup = draw_area_in(f.size(), 30, 10);
                f.render_widget(Clear, popup);
                self.application.view(&Id::QuitPopup, f, popup);
            } else if self.application.mounted(&Id::ErrorPopup) {
                let popup = draw_area_in(f.size(), 50, 15);
                f.render_widget(Clear, popup);
                self.application.view(&Id::ErrorPopup, f, popup);
            }
        });
    }

    /// Mount error and give focus to it
    pub(super) fn mount_error_popup(&mut self, err: impl ToString) {
        assert!(self
          .application
          .remount(
              Id::ErrorPopup,
              Box::new(ErrorPopup::new(err.to_string())),
              vec![]
          )
          .is_ok());
        assert!(self.application.active(&Id::ErrorPopup).is_ok());
    }

    /// finalize terminal
    pub(super) fn finalize_terminal(&mut self) {
        let _ = self.terminal.disable_raw_mode();
        let _ = self.terminal.leave_alternate_screen();
        let _ = self.terminal.clear_screen();
    }

    pub(super) fn subs() -> Vec<Sub<Id, AppEvent>> {
        vec![
            Sub::new(
                SubEventClause::Keyboard(KeyEvent {
                    code: Key::Esc,
                    modifiers: KeyModifiers::NONE,
                }),
                SubClause::Always,
            ),
        ]
    }


    /// Mount quit popup
    pub(super) fn mount_quit_popup(&mut self) {
        assert!(self
          .application
          .remount(Id::QuitPopup, Box::new(QuitPopup::default()), vec![])
          .is_ok());
        assert!(self.application.active(&Id::QuitPopup).is_ok());
    }

    pub(super) fn umount_quit_popup(&mut self) {
        let _ = self.application.umount(&Id::QuitPopup);
    }

    pub(super) fn umount_error_popup(&mut self) {
        let _ = self.application.umount(&Id::ErrorPopup);
    }
}

impl Update<Msg> for Ui {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        match msg.unwrap_or(Msg::None) {
            Msg::None => None,
            Msg::CloseErrorPopup => {
                self.umount_error_popup();
                None
            }
            Msg::Quit => Some(Msg::Quit),
            Msg::CloseQuitPopup => {
                self.umount_quit_popup();
                None
            }
            Msg::ShowQuitPopup =>  {
                self.mount_quit_popup();
                None
            },
            _ => None
        }
    }
}

/// Renders the user interface widgets.
pub fn render(app: &mut App, frame: &mut Frame) {
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    frame.render_widget(
        Paragraph::new(format!(
            "This is a tui template.\n\
                Press `Esc`, `Ctrl-C` or `q` to stop running.\n\
                Press left and right to increment and decrement the counter respectively.\n\
                Counter: {}",
            app.counter
        ))
        .block(
            Block::bordered()
                .title("Template")
                .title_alignment(Alignment::Center)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .centered(),
        frame.area(),
    )
}


