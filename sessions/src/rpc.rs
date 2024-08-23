//! The crate exports common types used when interacting with messages between
//! clients. This also includes communication over HTTP between relays.

mod params;

use {
    serde::{Deserialize, Serialize},
    std::{fmt::Debug, sync::Arc},
};

use crate::domain::{MessageId, Topic};
use crate::transport::Wired;
pub use params::*;

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

pub struct SdkError<'a> {
    code: u64,
    message: &'a str,
}

impl<'a> From<SdkError<'a>> for PairDeleteRequest {
    fn from(value: SdkError<'a>) -> Self {
        Self {
            code: value.code,
            message: String::from(value.message),
        }
    }
}

impl<'a> From<SdkError<'a>> for ErrorParams {
    fn from(value: SdkError) -> Self {
        Self {
            code: Some(value.code),
            message: String::from(value.message),
        }
    }
}

/* ----- INVALID (1xxx) ----- */
pub const INVALID_METHOD: SdkError = SdkError {
    message: "Invalid method.",
    code: 1001,
};

pub const INVALID_EVENT: SdkError = SdkError {
    message: "Invalid event.",
    code: 1002,
};

pub const INVALID_UPDATE_REQUEST: SdkError = SdkError {
    message: "Invalid update request.",
    code: 1003,
};
pub const INVALID_EXTEND_REQUEST: SdkError = SdkError {
    message: "Invalid extend request.",
    code: 1004,
};
pub const INVALID_SESSION_SETTLE_REQUEST: SdkError = SdkError {
    message: "Invalid session settle request.",
    code: 1005,
};
/* ----- UNAUTHORIZED (3xxx) ----- */
pub const UNAUTHORIZED_METHOD: SdkError = SdkError {
    message: "Unauthorized method.",
    code: 3001,
};
pub const UNAUTHORIZED_EVENT: SdkError = SdkError {
    message: "Unauthorized event.",
    code: 3002,
};
pub const UNAUTHORIZED_UPDATE_REQUEST: SdkError = SdkError {
    message: "Unauthorized update request.",
    code: 3003,
};
pub const UNAUTHORIZED_EXTEND_REQUEST: SdkError = SdkError {
    message: "Unauthorized extend request.",
    code: 3004,
};
/* ----- REJECTED (5xxx) ----- */
pub const USER_REJECTED: SdkError = SdkError {
    message: "User rejected.",
    code: 5000,
};
pub const USER_REJECTED_CHAINS: SdkError = SdkError {
    message: "User rejected chains.",
    code: 5001,
};
pub const USER_REJECTED_METHODS: SdkError = SdkError {
    message: "User rejected methods.",
    code: 5002,
};
pub const USER_REJECTED_EVENTS: SdkError = SdkError {
    message: "User rejected events.",
    code: 5003,
};
pub const UNSUPPORTED_CHAINS: SdkError = SdkError {
    message: "Unsupported chains.",
    code: 5100,
};
pub const UNSUPPORTED_METHODS: SdkError = SdkError {
    message: "Unsupported methods.",
    code: 5101,
};
pub const UNSUPPORTED_EVENTS: SdkError = SdkError {
    message: "Unsupported events.",
    code: 5102,
};
pub const UNSUPPORTED_ACCOUNTS: SdkError = SdkError {
    message: "Unsupported accounts.",
    code: 5103,
};
pub const UNSUPPORTED_NAMESPACE_KEY: SdkError = SdkError {
    message: "Unsupported namespace key.",
    code: 5104,
};
/* ----- REASON (6xxx) ----- */
pub const USER_DISCONNECTED: SdkError = SdkError {
    message: "User disconnected.",
    code: 6000,
};
/* ----- FAILURE (7xxx) ----- */
pub const SESSION_SETTLEMENT_FAILED: SdkError = SdkError {
    message: "Session settlement failed.",
    code: 7000,
};
/* ----- PAIRING (10xxx) ----- */
pub const WC_METHOD_UNSUPPORTED: SdkError = SdkError {
    message: "Unsupported wc_ method.",
    code: 10001,
};
