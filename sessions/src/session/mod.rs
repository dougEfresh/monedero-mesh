use crate::rpc::{RequestParams, SessionDeleteRequest};
use crate::transport::SessionTransport;
use crate::{
    Cipher, Error, PairingManager, PairingTopic, SessionEventRequest, SessionHandler,
    SessionSettled, Topic,
};
use crate::{Result, SessionDeleteHandler};
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, warn};
use xtra::prelude::*;

mod pending;
mod session_delete;
mod session_ping;

use crate::actors::{ClearSession, SessionRequestHandlerActor};
use crate::crypto::CipherError;
pub(crate) use pending::PendingSession;
use walletconnect_namespaces::Namespaces;

/// https://specs.walletconnect.com/2.0/specs/clients/sign/session-proposal
///
/// New session as the result of successful session proposal.
#[derive(Clone, Actor)]
pub struct ClientSession {
    pub settled: Arc<SessionSettled>,
    transport: SessionTransport,
    session_actor: Address<SessionRequestHandlerActor>,
    tx: mpsc::UnboundedSender<SessionEventRequest>,
}

impl ClientSession {
    pub(crate) async fn new(
        session_actor: Address<SessionRequestHandlerActor>,
        transport: SessionTransport,
        settled: SessionSettled,
        tx: mpsc::UnboundedSender<SessionEventRequest>,
    ) -> Result<Self> {
        let me = Self {
            session_actor: session_actor.clone(),
            transport,
            settled: Arc::new(settled),
            tx,
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
}
