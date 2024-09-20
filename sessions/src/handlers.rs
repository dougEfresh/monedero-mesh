use crate::rpc::{
    Event, ResponseParamsSuccess, RpcResponsePayload, SessionDeleteRequest, SessionProposeRequest,
    SessionProposeResponse, SessionRequestRequest, SessionSettleRequest,
};
use crate::SocketEvent;
use async_trait::async_trait;
use monedero_namespaces::Namespaces;
use serde_json::json;

#[async_trait]
pub trait SocketListener: Sync + Send + 'static {
    async fn handle_socket_event(&self, _event: SocketEvent) {}
}

#[async_trait]
pub trait SessionEventHandler: Send + Sync + 'static {
    async fn event(&self, event: Event) {}
}

pub enum WalletRequestResponse {
    Success(serde_json::Value),
    Error(crate::rpc::SdkErrors),
}

#[async_trait]
pub trait SessionHandler: Send + Sync + 'static + SessionEventHandler {
    async fn request(&self, request: SessionRequestRequest) -> WalletRequestResponse;
}

#[async_trait]
pub trait WalletSettlementHandler: Send + Sync + 'static {
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
    async fn request(&self, request: SessionRequestRequest) -> WalletRequestResponse {
        tracing::info!("got session request {:#?}", request);
        WalletRequestResponse::Success(json!({}))
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
