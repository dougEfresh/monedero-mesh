use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread::sleep,
};

use crate::message::Msg;
use crate::{panic_set_hook};

use anyhow::Result;
use crossbeam::channel::Sender;
use tokio::time;

pub struct Tick {
  tx: Sender<Msg>,
  duration: time::Duration,
  is_terminated: Arc<AtomicBool>,
}

impl Tick {
  pub fn new(tx: Sender<Msg>, rate: time::Duration, is_terminated: Arc<AtomicBool>) -> Self {
    Self {
      tx,
      duration: rate,
      is_terminated,
    }
  }

  pub fn start(&self) -> Result<()> {
    tracing::info!("tick start");

    let ret = self.tick();

    self.is_terminated.store(true, Ordering::Relaxed);

    tracing::info!("tick end");

    ret
  }

  pub fn set_panic_hook(&self) {
    let is_terminated = self.is_terminated.clone();

    panic_set_hook!({
            is_terminated.store(true, Ordering::Relaxed);
        });
  }

  fn tick(&self) -> Result<()> {
    while !self.is_terminated.load(Ordering::Relaxed) {
      sleep(self.duration);

      self.tx.send(Msg::Tick)?;
    }

    Ok(())
  }
}
