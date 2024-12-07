mod actors;
mod dapp;
mod error;
pub mod handlers;
mod pair;
mod relay;
pub mod rpc;
pub mod session;
mod transport;
mod wait;
mod wallet;

use {
    crate::rpc::SessionRequestRequest,
    monedero_domain::{namespaces::Event, Topic},
    pin_project_lite::pin_project,
    std::{
        fmt::{Display, Formatter},
        future::Future,
        pin::Pin,
        sync::Once,
        task::{Context, Poll},
    },
    tokio::sync::oneshot,
};
pub use {
    crate::session::ClientSession,
    actors::{Actors, RegisteredComponents},
    dapp::Dapp,
    error::Error,
    handlers::*,
    monedero_domain as domain,
    monedero_relay::{
        auth_token,
        default_connection_opts,
        mock_connection_opts,
        ClientError,
        MockRelay,
    },
    monedero_store::{Error as StoreError, KvStorage},
    pair::{PairingManager, WalletConnectBuilder},
    rpc::{Metadata, SdkErrors},
    wallet::Wallet,
};

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
        crate::INIT,
        tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
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
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static + Send,
{
    tokio::spawn(future);
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn spawn_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}
