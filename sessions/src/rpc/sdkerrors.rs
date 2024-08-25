use crate::rpc::{ErrorParams, PairDeleteRequest};

pub enum SdkErrors {
    InvalidMethod,
    InvalidEvent,
    InvalidUpdateRequest,
    InvalidExtendRequest,
    InvalidSessionSettleRequest,
    UnauthorizedMethod,
    UnauthorizedEvent,
    UnauthorizedUpdateRequest,
    UnauthorizedExtendRequest,
    UserRejected,
    UserRejectedChains,
    UserRejectedMethods,
    UserRejectedEvents,
    UnsupportedChains,
    UnsupportedMethods,
    UnsupportedEvents,
    UnsupportedAccounts,
    UnsupportedNamespaceKey,
    UserDisconnected,
    SessionSettlementFailed,
    WcMethodUnsupported,
}

impl<'a> From<SdkErrors> for SdkError<'_> {
    fn from(value: SdkErrors) -> Self {
        match value {
            SdkErrors::InvalidMethod => INVALID_METHOD,
            SdkErrors::InvalidEvent => INVALID_EVENT,
            SdkErrors::InvalidUpdateRequest => INVALID_UPDATE_REQUEST,
            SdkErrors::InvalidExtendRequest => INVALID_EXTEND_REQUEST,
            SdkErrors::InvalidSessionSettleRequest => INVALID_SESSION_SETTLE_REQUEST,
            SdkErrors::UnauthorizedMethod => UNAUTHORIZED_METHOD,
            SdkErrors::UnauthorizedEvent => UNAUTHORIZED_EVENT,
            SdkErrors::UnauthorizedUpdateRequest => UNAUTHORIZED_UPDATE_REQUEST,
            SdkErrors::UnauthorizedExtendRequest => UNAUTHORIZED_EXTEND_REQUEST,
            SdkErrors::UserRejected => USER_REJECTED,
            SdkErrors::UserRejectedChains => USER_REJECTED_CHAINS,
            SdkErrors::UserRejectedMethods => USER_REJECTED_METHODS,
            SdkErrors::UserRejectedEvents => USER_REJECTED_EVENTS,
            SdkErrors::UnsupportedChains => UNSUPPORTED_CHAINS,
            SdkErrors::UnsupportedMethods => UNSUPPORTED_METHODS,
            SdkErrors::UnsupportedEvents => UNSUPPORTED_EVENTS,
            SdkErrors::UnsupportedAccounts => UNSUPPORTED_ACCOUNTS,
            SdkErrors::UnsupportedNamespaceKey => UNSUPPORTED_NAMESPACE_KEY,
            SdkErrors::UserDisconnected => USER_DISCONNECTED,
            SdkErrors::SessionSettlementFailed => SESSION_SETTLEMENT_FAILED,
            SdkErrors::WcMethodUnsupported => WC_METHOD_UNSUPPORTED,
        }
    }
}

pub struct SdkError<'a> {
    pub code: u64,
    pub message: &'a str,
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
