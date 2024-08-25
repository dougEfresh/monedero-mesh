//! The crate exports common types used when interacting with messages between
//! clients. This also includes communication over HTTP between relays.

mod params;
mod sdkerrors;

use std::future::Future;
use {
    serde::{Deserialize, Serialize},
    std::{fmt::Debug, sync::Arc},
};

use crate::domain::{MessageId, Topic};
pub use params::*;
pub use sdkerrors::SdkErrors;

/// Version of the WalletConnect protocol that we're implementing.
pub const JSON_RPC_VERSION_STR: &str = "2.0";
pub static JSON_RPC_VERSION: once_cell::sync::Lazy<Arc<str>> =
    once_cell::sync::Lazy::new(|| Arc::from(JSON_RPC_VERSION_STR));

/// Errors covering payload validation problems.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid request ID")]
    RequestId,

    #[error("Invalid JSON RPC version")]
    JsonRpcVersion,
}

/// Errors caught while processing the Sign API request/response. These should
/// be specific enough for the clients to make sense of the problem.
#[derive(Debug, thiserror::Error)]
pub enum GenericError {
    /// Request parameters validation failed.
    #[error("Request validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Request/response serialization error.
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transport {
    Request(RequestParams),
    Response(ResponseParams),
}

/// Enum representing a JSON RPC payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Payload {
    Request(Request),
    Response(Response),
}

impl From<Request> for Payload {
    fn from(value: Request) -> Self {
        Payload::Request(value)
    }
}

impl From<Response> for Payload {
    fn from(value: Response) -> Self {
        Payload::Response(value)
    }
}

impl Payload {
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::Request(request) => request.validate(),
            Self::Response(response) => response.validate(),
        }
    }

    pub fn irn_tag_in_range(tag: u32) -> bool {
        (1000..=1115).contains(&tag)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RpcResponse {
    pub(crate) id: MessageId,
    pub(crate) topic: Topic,
    pub(crate) payload: RpcResponsePayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpcErrorResponse {
    pub(crate) id: MessageId,
    pub(crate) topic: Topic,
    pub(crate) payload: ErrorParams,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpcResponsePayload {
    Success(ResponseParamsSuccess),
    Error(ResponseParamsError),
}

impl RpcResponse {
    pub(crate) fn unknown(id: MessageId, topic: Topic, params: ResponseParamsError) -> Self {
        Self {
            id,
            topic,
            payload: RpcResponsePayload::Error(params),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RpcRequest {
    pub(crate) topic: Topic,
    pub(crate) payload: Request,
}

/// Data structure representing a JSON RPC request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// ID this message corresponds to.
    pub id: MessageId,

    /// The JSON RPC version.
    pub jsonrpc: Arc<str>,

    /// The parameters required to fulfill this request.
    #[serde(flatten)]
    pub params: RequestParams,
}

impl Request {
    /// Create a new instance.
    pub fn new(id: MessageId, params: RequestParams) -> Self {
        Self {
            id,
            jsonrpc: JSON_RPC_VERSION_STR.into(),
            params,
        }
    }

    /// Validates the request payload.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.jsonrpc.as_ref() != JSON_RPC_VERSION_STR {
            return Err(ValidationError::JsonRpcVersion);
        }

        Ok(())
    }
}

/// Data structure representing JSON RPC response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    /// ID this message corresponds to.
    pub id: MessageId,

    /// RPC version.
    pub jsonrpc: Arc<str>,

    /// The parameters required to fulfill this response.
    #[serde(flatten)]
    pub params: ResponseParams,
}

impl Response {
    /// Create a new instance.
    pub fn new(id: MessageId, params: ResponseParams) -> Self {
        Self {
            id,
            jsonrpc: JSON_RPC_VERSION.clone(),
            params,
        }
    }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.jsonrpc.as_ref() != JSON_RPC_VERSION_STR {
            return Err(ValidationError::JsonRpcVersion);
        }

        Ok(())
    }
}

use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot;

pin_project! {
    pub struct ProposeFuture<T> {
        #[pin]
        receiver: oneshot::Receiver<T>,
    }
}

impl<T> ProposeFuture<T> {
    pub fn new(receiver: oneshot::Receiver<T>) -> Self {
        Self { receiver }
    }
}

impl<T> Future for ProposeFuture<T> {
    type Output = Result<T, oneshot::error::RecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().receiver.poll(cx)
    }
}
