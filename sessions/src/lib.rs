mod actors;
mod dapp;
mod error;
pub mod handlers;
mod pair;
mod relay;
pub mod rpc;
pub mod session;
mod transport;
mod wallet;
pub use {
    crate::session::ClientSession,
    dapp::Dapp,
    error::Error,
    handlers::*,
    pair::{PairingManager, WalletConnectBuilder},
    wallet::Wallet,
};
use {
    pin_project_lite::pin_project,
    serde::{Deserialize, Serialize},
    std::{
        fmt::{Display, Formatter},
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex, Once},
        task::{Context, Poll},
    },
    tokio::sync::oneshot,
};
pub type Atomic<T> = Arc<Mutex<T>>;
use {
    crate::rpc::SessionRequestRequest,
    monedero_domain::namespaces::Event,
};
pub use {
    actors::{Actors, RegisteredComponents},
    monedero_relay::ClientError,
    rpc::{Metadata, SdkErrors},
};

pub use monedero_relay::auth_token;
use monedero_domain::{SessionTopic, Topic};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum SocketEvent {
    Connected,
    #[default]
    Disconnect,
    ForceDisconnect,
}

impl Display for SocketEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected => {
                write!(f, "connected")
            }
            Self::Disconnect => {
                write!(f, "disconnected")
            }
            Self::ForceDisconnect => {
                write!(f, "force disconnect")
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
#[allow(dead_code)]
static INIT: Once = Once::new();

pin_project! {
    pub struct ProposeFuture {
        #[pin]
        receiver: oneshot::Receiver<Result<ClientSession>>,
    }
}

impl ProposeFuture {
    #[must_use]
    pub fn new(receiver: oneshot::Receiver<Result<ClientSession>>) -> Self {
        Self { receiver }
    }
}

impl Future for ProposeFuture {
    type Output = Result<ClientSession>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().receiver.poll(cx) {
            Poll::Ready(Ok(value)) => Poll::Ready(value),
            Poll::Ready(Err(_)) => Poll::Ready(Err(Error::ReceiveError)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub enum SessionEventRequest {
    Event(Event),
    Request(SessionRequestRequest),
}

pub(crate) fn shorten_topic(id: &Topic) -> String {
    let mut id = format!("{id}");
    if id.len() > 10 {
        id = String::from(&id[0..9]);
    }
    id
}

#[cfg(test)]
pub(crate) mod test {
    use {
        crate::{NoopSessionHandler, SessionHandler, INIT, rpc::Event},
        std::{sync::Arc, time::Duration},
        tokio::sync::Mutex,
        tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
        xtra::prelude::*,
    };

    pub(crate) fn init_tracing() {
        INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_target(true)
                .with_level(true)
                .with_span_events(FmtSpan::CLOSE)
                .with_env_filter(EnvFilter::from_default_env())
                .init();
        });
    }

    #[derive(Clone, Actor)]
    struct Actor1 {
        handlers: Arc<Mutex<Vec<Box<dyn SessionHandler>>>>,
    }

    #[derive(Actor)]
    struct Actor2 {}

    #[derive(Clone)]
    struct Dummy;

    impl Handler<Box<dyn SessionHandler>> for Actor1 {
        type Return = ();

        async fn handle(
            &mut self,
            message: Box<dyn SessionHandler>,
            _ctx: &mut Context<Self>,
        ) -> Self::Return {
            self.handlers.lock().await.push(message);
        }
    }

    impl Actor1 {
        async fn handle_event(&self, event: Event) {
            let l = self.handlers.lock().await;
            for h in l.iter() {
                h.event(event.clone()).await;
            }
        }
    }

    impl Handler<Event> for Actor1 {
        type Return = ();

        async fn handle(&mut self, message: Event, _ctx: &mut Context<Self>) -> Self::Return {
            let me = self.clone();
            tokio::spawn(async move {
                me.handle_event(message).await;
            });
        }
    }

    impl Handler<Dummy> for Actor1 {
        type Return = ();

        async fn handle(&mut self, message: Dummy, _ctx: &mut Context<Self>) -> Self::Return {
            tracing::info!("Actor1 got message");
        }
    }

    impl Handler<Dummy> for Actor2 {
        type Return = ();

        async fn handle(&mut self, message: Dummy, _ctx: &mut Context<Self>) -> Self::Return {
            tracing::info!("Actor2 got message");
        }
    }
    
    /*
    #[tokio::test]
    async fn test_actor_broadcast() -> anyhow::Result<()> {
        init_tracing();
        let handlers: Arc<Mutex<Vec<Box<dyn SessionHandler>>>> =
            Arc::new(Mutex::new(vec![Box::new(NoopSessionHandler {})]));
        let boxed: Box<dyn SessionHandler> = Box::new(NoopSessionHandler {});
        let act = Actor1 { handlers };
        let a1 = xtra::spawn_tokio(act.clone(), Mailbox::unbounded());
        a1.send(Dummy).await?;
        a1.send(boxed).await?;
        a1.send(Event::AccountsChanged).await?;
        // a2.broadcast(Dummy).await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        eprintln!("size {}", act.handlers.lock().await.len());
        Ok(())
    }
     */
}
