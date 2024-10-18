//! https://specs.walletconnect.com/2.0/specs/clients/core/pairing/rpc-methods
//! #wc_pairingPing

use serde::{Deserialize, Serialize};

use super::IrnMetadata;
use crate::rpc::{
    ErrorParams, IntoUnknownError, ResponseParamsError, TAG_PAIR_PING_REQUEST,
    TAG_PAIR_PING_RESPONSE,
};

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: TAG_PAIR_PING_REQUEST,
    ttl: 30,
    prompt: false,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: TAG_PAIR_PING_RESPONSE,
    ttl: 30,
    prompt: false,
};

#[derive(Debug, Default, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PairPingRequest {}

impl IntoUnknownError for PairPingRequest {
    fn unknown(&self) -> ResponseParamsError {
        ResponseParamsError::PairPing(ErrorParams::unknown())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::super::tests::param_serde_test;
    use super::*;

    #[test]
    fn test_serde_pair_ping_request() -> Result<()> {
        let json = r#"{}"#;

        param_serde_test::<PairPingRequest>(json)
    }
}
