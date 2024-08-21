use crate::domain::{MessageId, SubscriptionId, Topic};
use crate::relay::{Client, MessageIdGenerator};
use crate::rpc::{
    Payload, RelayProtocolMetadata, Request, RequestParams, Response, ResponseParams,
    ResponseParamsSuccess, SessionDeleteRequest, SessionEventRequest, SessionExtendRequest,
    SessionProposeRequest, SessionRequestRequest, SessionSettleRequest, SessionUpdateRequest,
};
use crate::{Atomic, Cipher, Settlement};
use crate::{Message, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{debug, error, info, warn, Instrument};

pub trait Wired: Debug + Clone + PartialEq + Serialize + DeserializeOwned {}
impl<T> Wired for T where T: Debug + Clone + PartialEq + Serialize + DeserializeOwned {}

/// RpcRecv embeds the topic and id to all events. This gives listeners context about the broadcast [WireEvent](WireEvent)
#[derive(Clone, Debug, Serialize)]
pub struct RpcRecv<T: Wired> {
    pub id: MessageId,
    pub topic: Topic,
    pub recv: T,
}

impl<T: Wired> RpcRecv<T> {
    pub fn new(id: MessageId, topic: Topic, recv: T) -> Self {
        Self { id, topic, recv }
    }
}

/// Sign API request parameters.
///
/// https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods
/// https://specs.walletconnect.com/2.0/specs/clients/sign/data-structures
#[derive(Clone, Debug, Serialize)]
pub enum SessionRpcEvent {
    Propose(RpcRecv<SessionProposeRequest>),
    Settle(RpcRecv<SessionSettleRequest>),
    Update(RpcRecv<SessionUpdateRequest>),
    Extend(RpcRecv<SessionExtendRequest>),
    Request(RpcRecv<SessionRequestRequest>),
    Event(RpcRecv<SessionEventRequest>),
    Delete(RpcRecv<SessionDeleteRequest>),
    Ping(RpcRecv<()>),
    Unknown(RpcRecv<()>),
}

/// Pairing RPC methods
/// https://specs.walletconnect.com/2.0/specs/clients/core/pairing/rpc-methods
#[derive(Clone, Debug, Serialize)]
pub enum PairingRpcEvent {
    Delete(RpcRecv<()>),
    Extend(RpcRecv<()>),
    Ping(RpcRecv<()>),
    Unknown(RpcRecv<()>),
}

impl From<RpcRecv<RequestParams>> for SessionRpcEvent {
    fn from(value: RpcRecv<RequestParams>) -> Self {
        match value.recv {
            RequestParams::SessionPropose(p) => {
                SessionRpcEvent::Propose(RpcRecv::new(value.id, value.topic, p))
            }
            RequestParams::SessionSettle(p) => {
                SessionRpcEvent::Settle(RpcRecv::new(value.id, value.topic, p))
            }
            RequestParams::SessionUpdate(p) => {
                SessionRpcEvent::Update(RpcRecv::new(value.id, value.topic, p))
            }
            RequestParams::SessionExtend(p) => {
                SessionRpcEvent::Extend(RpcRecv::new(value.id, value.topic, p))
            }
            RequestParams::SessionRequest(p) => {
                SessionRpcEvent::Request(RpcRecv::new(value.id, value.topic, p))
            }
            RequestParams::SessionEvent(p) => {
                SessionRpcEvent::Event(RpcRecv::new(value.id, value.topic, p))
            }
            RequestParams::SessionDelete(p) => {
                SessionRpcEvent::Delete(RpcRecv::new(value.id, value.topic, p))
            }
            RequestParams::SessionPing(p) => {
                SessionRpcEvent::Ping(RpcRecv::new(value.id, value.topic, p))
            }
            _ => SessionRpcEvent::Unknown(RpcRecv::new(value.id, value.topic, ())),
        }
    }
}

impl From<RpcRecv<RequestParams>> for PairingRpcEvent {
    fn from(value: RpcRecv<RequestParams>) -> Self {
        match value.recv {
            RequestParams::PairDelete(_) => {
                PairingRpcEvent::Delete(RpcRecv::new(value.id, value.topic, ()))
            }
            RequestParams::PairExtend(_) => {
                PairingRpcEvent::Extend(RpcRecv::new(value.id, value.topic, ()))
            }
            RequestParams::PairPing(_) => {
                PairingRpcEvent::Ping(RpcRecv::new(value.id, value.topic, ()))
            }
            _ => PairingRpcEvent::Unknown(RpcRecv::new(value.id, value.topic, ())),
        }
    }
}

// A collection of low level events (socket) and high level decrypted session/pairing RPC calls
#[derive(Clone, Debug, Serialize)]
pub enum WireEvent {
    Connected,
    Disconnect,
    // Raw message before decrypt
    MessageRecv(Message),
    // Decrypted messages
    RequestRecv(RpcRecv<RequestParams>),
    ResponseRecv(RpcRecv<ResponseParams>),

    // Replay back to a request
    SendResponse(RpcRecv<ResponseParamsSuccess>),

    // Rpc Session requests
    SessionRpc(SessionRpcEvent),

    // Pairing topic events
    PairingRpc(PairingRpcEvent),

    // Final settlement
    //SessionSettled(SessionTransport),

    // Errors
    DisconnectFromHandler,
    DecryptError(Message, Arc<str>),
    SettlementFailed,

    Shutdown,
}

#[derive(Clone)]
pub(crate) struct TopicTransport {
    pending_requests: PendingRequests,
    ciphers: Cipher,
    relay: Client,
}

impl TopicTransport {
    pub(crate) fn new(pending_requests: PendingRequests, ciphers: Cipher, client: Client) -> Self {
        Self {
            pending_requests,
            ciphers,
            relay: client,
        }
    }

    pub async fn publish_response(
        &self,
        id: MessageId,
        topic: Topic,
        resp: ResponseParamsSuccess,
    ) -> Result<()> {
        let irn_metadata = resp.irn_metadata();
        let response = Response::new(id, resp.try_into()?);
        let encrypted = self.ciphers.encode(&topic, &response)?;
        self.relay
            .publish(
                topic,
                Arc::from(encrypted),
                irn_metadata.tag,
                Duration::from_secs(irn_metadata.ttl),
                irn_metadata.prompt,
            )
            .await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn publish_request<R: DeserializeOwned>(
        &self,
        topic: Topic,
        params: RequestParams,
    ) -> Result<R> {
        let (id, rx) = self.pending_requests.add()?;
        let irn_metadata = params.irn_metadata();
        let request = Request::new(id, params);
        info!("Sending request topic={topic}");
        let encrypted = self.ciphers.encode(&topic, &request)?;
        let ttl = Duration::from_secs(irn_metadata.ttl);
        self.relay
            .publish(
                topic,
                Arc::from(encrypted),
                irn_metadata.tag,
                ttl,
                irn_metadata.prompt,
            )
            .await?;

        match timeout(ttl, rx).await {
            Err(_) => Err(crate::Error::SessionRequestTimeout),
            Ok(res) => match res {
                Ok(v) => match v {
                    Ok(value) => Ok(serde_json::from_value(value)?),
                    Err(err) => Err(err),
                },
                Err(_) => Err(crate::Error::ResponseChannelError(id)),
            },
        }
    }
}

#[derive(Clone)]
pub(crate) struct SessionTransport {
    pub(crate) topic: Topic,
    pub(crate) transport: TopicTransport,
}

impl SessionTransport {
    pub async fn publish_response(&self, id: MessageId, resp: ResponseParamsSuccess) -> Result<()> {
        self.transport
            .publish_response(id, self.topic.clone(), resp)
            .await
    }
    pub async fn publish_request<R: DeserializeOwned>(&self, params: RequestParams) -> Result<R> {
        self.transport
            .publish_request(self.topic.clone(), params)
            .await
    }
}

#[derive(Clone)]
pub(crate) struct PendingRequests {
    requests: Atomic<HashMap<MessageId, oneshot::Sender<Result<Value>>>>,
    generator: MessageIdGenerator,
}

impl PendingRequests {
    pub fn new() -> Self {
        Self {
            //settlement: Arc::new(Mutex::new(HashMap::new())),
            requests: Arc::new(Mutex::new(HashMap::new())),
            generator: MessageIdGenerator::new(),
        }
    }
}

impl PendingRequests {
    pub fn add(&self) -> Result<(MessageId, oneshot::Receiver<Result<Value>>)> {
        let id = self.generator.next();
        let (tx, rx) = oneshot::channel::<Result<Value>>();
        {
            let mut pending_requests = self.requests.lock().unwrap();
            pending_requests.insert(id, tx);
        }
        Ok((id, rx))
    }

    #[tracing::instrument(level = "debug", skip(self, params))]
    pub fn handle_response(&self, id: &MessageId, params: ResponseParams) {
        match self.requests.lock() {
            Ok(mut l) => match l.remove(id) {
                Some(sender) => {
                    let res: Result<Value> = match params {
                        ResponseParams::Success(v) => Ok(v),
                        ResponseParams::Err(v) => Err(crate::Error::RpcError(v)),
                    };
                    let _ = sender.send(res);
                }
                None => {
                    error!("no matching id {id} to handle {params:#?}");
                }
            },
            Err(_) => {
                warn!("poison lock!!");
            }
        };
    }
}
