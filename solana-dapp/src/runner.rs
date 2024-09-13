use std::{
  sync::{atomic::AtomicBool, Arc},
  thread, time,
};

use anyhow::{Context as _, Result};
use crossbeam::channel::{bounded, Receiver, Sender};
use ratatui::layout::Direction;
use crate::{
  message::Msg,
  workers::{Tick, UserInput},
};
use crate::workers::Render;

pub struct Runner;

impl Runner {
  pub fn run() -> Result<()> {

    let (tx_input, rx_main): (Sender<Msg>, Receiver<Msg>) = bounded(128);
    let (tx_main, rx_dapp): (Sender<Msg>, Receiver<Msg>) = bounded(256);
    let tx_tick = tx_input.clone();

    let is_terminated = Arc::new(AtomicBool::new(false));

    let user_input = UserInput::new(tx_input.clone(), is_terminated.clone());

    let tick = Tick::new(
      tx_tick.clone(),
      time::Duration::from_millis(200),
      is_terminated.clone(),
    );
    let render = Render::new(
      tx_main.clone(),
      rx_main.clone(),
      is_terminated.clone(),
      Direction::Horizontal,
    );

    thread::scope(|s| {

      let tick_handler = s.spawn(move || {
        tick.set_panic_hook();
        tick.start()
      });

      let user_input_handler = s.spawn(move || {
        user_input.set_panic_hook();
        user_input.start()
      });

      let render_handler = s.spawn(move || {
        render.set_panic_hook();
        render.start()
      });

      tick_handler
        .join()
        .expect("tick thread panicked")
        .context("tick thread error")?;

      user_input_handler
        .join()
        .expect("user_input thread panicked")
        .context("user_input thread error")?;

      render_handler
        .join()
        .expect("render thread panicked")
        .context("render thread error")?;

      anyhow::Ok(())

    })?;

    Ok(())
  }
}