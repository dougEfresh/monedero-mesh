use crate::rpc::{
    Event, ResponseParamsSuccess, RpcResponsePayload, SessionDeleteRequest, SessionProposeRequest,
    SessionProposeResponse, SessionRequestRequest, SessionSettleRequest,
};
use crate::SocketEvent;
use async_trait::async_trait;
use walletconnect_namespaces::Namespaces;

#[async_trait]
pub trait SocketListener: Sync + Send + 'static {
    async fn handle_socket_event(&self, _event: SocketEvent) {}
}

#[async_trait]
pub trait SessionEventHandler: Send + Sync + 'static {
    async fn event(&self, _event: Event) {}
}

#[async_trait]
pub trait SessionHandler: Send + Sync + 'static + SessionEventHandler {
    async fn request(&self, _request: SessionRequestRequest) {}
}

#[async_trait]
pub trait WalletProposalHandler: Send + Sync + 'static {
    async fn settlement(&self, proposal: SessionProposeRequest)
        -> Result<Namespaces, crate::Error>;

    async fn verify_settlement(
        &self,
        proposal: SessionProposeRequest,
        pk: String,
    ) -> (bool, RpcResponsePayload) {
        let result = RpcResponsePayload::Success(ResponseParamsSuccess::SessionPropose(
            SessionProposeResponse {
                relay: Default::default(),
                responder_public_key: pk,
            },
        ));
        (true, result)
    }
}

pub struct NoopSessionHandler;

#[async_trait]
impl SessionEventHandler for NoopSessionHandler {
    async fn event(&self, event: Event) {
        tracing::info!("got session event {event:#?}");
    }
}

impl SocketListener for NoopSessionHandler {}

#[async_trait]
impl SessionHandler for NoopSessionHandler {
    async fn request(&self, request: SessionRequestRequest) {
        tracing::info!("got session request {:#?}", request);
    }
}

pub struct NoopSessionDeleteHandler;
impl SessionDeleteHandler for NoopSessionDeleteHandler {}

#[async_trait]
pub trait SessionDeleteHandler: Send + Sync + 'static {
    async fn handle(&self, request: SessionDeleteRequest) {
        tracing::info!("Session delete request {:#?}", request);
    }
}
