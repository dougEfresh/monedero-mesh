use std::{
  cell::RefCell,
  io::{self},
  rc::Rc,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use ratatui::{backend::CrosstermBackend, layout::Direction, Terminal, TerminalOptions, Viewport};
use crate::app::App;
use crate::message::{Msg, UserEvent};
use crate::panic_set_hook;

pub struct Render {
  tx: Sender<Msg>,
  rx: Receiver<Msg>,
  is_terminated: Arc<AtomicBool>,
  direction: Direction,
}

impl Render {
  pub fn new(
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    is_terminated: Arc<AtomicBool>,
    direction: Direction,
  ) -> Self {
    Self {
      direction,
      tx,
      rx,
      is_terminated,
    }
  }

  pub fn start(self) -> Result<()> {
    tracing::info!("render start");

    let ret = self.render();

    self.is_terminated.store(true, Ordering::Relaxed);

    tracing::info!("render end");

    ret
  }

  pub fn set_panic_hook(&self) {
    let is_terminated = self.is_terminated.clone();

    panic_set_hook!({
            is_terminated.store(true, Ordering::Relaxed);
        });
  }

  fn render(&self) -> Result<()> {
    let mut app = App::new();
    let mut terminal = Terminal::with_options(
      CrosstermBackend::new(io::stdout()),
      TerminalOptions {
        viewport: Viewport::Fullscreen,
      },
    )?;
    while !self.is_terminated.load(Ordering::Relaxed) {
      terminal.draw(|f| {
        crate::ui::render(&mut app, f);
      })?;
      match self.rx.recv().expect("Failed to recv") {
        Msg::Dapp(_) => {}
        Msg::User(UserEvent::Key(k))  => {
          crate::handler::handle_key_events(k, &mut app);
          if !app.running {
            self.is_terminated.store(true, Ordering::Relaxed);
          }
        }
        Msg::User(_)  => {}
        Msg::Tick => {}
        _ => {
          break
        }
      }
    }
    Ok(())
  }
}
