use {
    crate::{
        rpc::{RequestParams, SessionDeleteRequest},
        transport::SessionTransport,
        Error,
        Result,
        SessionHandler,
        Topic,
    },
    monedero_domain::SessionSettled,
    serde::de::DeserializeOwned,
    std::{
        fmt::{Debug, Display, Formatter},
        sync::Arc,
        time::Duration,
    },
    tokio::sync::Mutex,
    tracing::{error, warn},
    xtra::prelude::*,
};

mod pending;
mod session_delete;
mod session_ping;
mod session_request;

pub(crate) use pending::PendingSession;
use {
    crate::actors::{ClearSession, SessionRequestHandlerActor},
    monedero_cipher::CipherError,
    monedero_domain::namespaces::Namespaces,
};

#[derive(Clone, Hash, Eq, PartialEq)]
pub(crate) enum Category {
    Dapp,
    Wallet,
}

impl Category {
    fn fmt_common(&self) -> String {
        match self {
            Self::Dapp => String::from("[dapp]"),
            Self::Wallet => String::from("[wallet"),
        }
    }
}

impl Debug for Category {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}

impl Display for Category {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_common())
    }
}

/// https://specs.walletconnect.com/2.0/specs/clients/sign/session-proposal
///
/// New session as the result of successful session proposal.
#[derive(Clone, Actor)]
pub struct ClientSession {
    pub settled: Arc<SessionSettled>,
    transport: SessionTransport,
    session_actor: Address<SessionRequestHandlerActor>,
    handler: Arc<Mutex<Box<dyn SessionHandler>>>,
    category: Category,
}

impl Debug for ClientSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} topic:{}",
            self.category,
            crate::shorten_topic(&self.topic())
        )
    }
}

impl ClientSession {
    pub(crate) async fn new(
        session_actor: Address<SessionRequestHandlerActor>,
        transport: SessionTransport,
        settled: SessionSettled,
        handler: Arc<Mutex<Box<dyn SessionHandler>>>,
        category: Category,
    ) -> Result<Self> {
        let me = Self {
            session_actor,
            transport,
            settled: Arc::new(settled),
            handler,
            category,
        };
        me.register().await?;
        Ok(me)
    }
}

impl ClientSession {
    async fn register(&self) -> Result<()> {
        self.session_actor.send(self.clone()).await?;
        Ok(())
    }

    pub fn namespaces(&self) -> &Namespaces {
        &self.settled.namespaces
    }

    pub fn topic(&self) -> Topic {
        self.transport.topic.clone()
    }

    pub async fn publish_request<R: DeserializeOwned>(&self, params: RequestParams) -> Result<R> {
        match self.transport.publish_request(params).await {
            Ok(r) => Ok(r),
            Err(Error::CipherError(CipherError::UnknownTopic(_))) => {
                Err(Error::NoClientSession(self.topic()))
            }
            Err(e) => Err(e),
        }
    }

    pub async fn ping(&self) -> Result<bool> {
        self.publish_request(RequestParams::SessionPing(())).await
    }

    pub async fn delete(&self) -> bool {
        let accepted: bool = match self
            .publish_request(RequestParams::SessionDelete(SessionDeleteRequest::default()))
            .await
        {
            Ok(false) => {
                warn!("other side did not accept our delete request");
                false
            }
            Ok(true) => true,
            Err(e) => {
                error!("failed send session delete: {e}");
                false
            }
        };
        let _ = self
            .session_actor
            .send(ClearSession(self.transport.topic.clone()))
            .await;
        accepted
    }

    pub async fn pinger(&self, duration: Duration) {
        let me = self.clone();
        loop {
            if let Err(e) = me.ping().await {
                warn!("pair ping failed! {e}");
            }
            tokio::time::sleep(duration).await;
        }
    }
}
