use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::info;
use xtra::prelude::*;
use xtra::WeakAddress;
use crate::{Atomic, SocketEvent, SocketHandler};


#[derive(Clone, Default, xtra::Actor)]
pub (crate) struct SocketLogActor {}

#[derive(Clone, Default, xtra::Actor)]
pub (crate) struct SocketActors
{
  subscribers: Arc<RwLock<Vec<Box<dyn SocketHandler + Send>>>>,
}

pub struct Subscribe<T: SocketHandler>(pub T);

impl<T: SocketHandler> Handler<Subscribe<T>> for SocketActors {
  type Return = ();

  async fn handle(&mut self, message: Subscribe<T>, ctx: &mut Context<Self>) -> Self::Return {
    self.subscribers.write().await.push(Box::new(message.0));
  }
}

impl Handler<SocketEvent> for SocketActors {
  type Return = ();

  async fn handle(&mut self, message: SocketEvent, ctx: &mut Context<Self>) -> Self::Return {
    let mut s = self.subscribers.write().await;
    tracing::trace!("Handling socket state change {message} for {} handlers", s.len());
    for handler in s.iter_mut() {
      handler.event(message.clone());
    }
  }
}

impl SocketHandler for SocketLogActor {
  fn event(&mut self, event: SocketEvent) {
    info!("[SocketLogActor] socket {event}");
  }
}

#[cfg(test)]
mod test {
  use std::time::Duration;
  use xtra::Mailbox;
  use super::*;

  struct Dummy;

  #[tokio::test]
  async fn test_add_socket_handler() -> anyhow::Result<()> {
    crate::tests::init_tracing();
    let socket_subscriber = xtra::spawn_tokio(SocketActors::default(), Mailbox::unbounded());
    let log_actor = Subscribe(SocketLogActor::default());
    socket_subscriber.send(log_actor).await?;


    tokio::spawn(async move {
      socket_subscriber.send(SocketEvent::Connected).await.unwrap();
    });
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
  }
}

