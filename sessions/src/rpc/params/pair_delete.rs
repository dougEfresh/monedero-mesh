//! https://specs.walletconnect.com/2.0/specs/clients/core/pairing/rpc-methods

use serde::{Deserialize, Serialize};

use super::IrnMetadata;
use crate::rpc::{ErrorParams, IntoUnknownError, ResponseParamsError};

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: crate::rpc::TAG_PAIR_DELETE_REQUEST,
    ttl: 30, // 86400 https://specs.walletconnect.com/2.0/specs/clients/core/pairing/rpc-methods#wc_pairingdelete
    prompt: false,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: crate::rpc::TAG_PAIR_DELETE_RESPONSE,
    ttl: 30, // 86400 https://specs.walletconnect.com/2.0/specs/clients/core/pairing/rpc-methods#wc_pairingdelete
    prompt: false,
};

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PairDeleteRequest {
    pub code: i64,
    pub message: String,
}

impl Default for PairDeleteRequest {
    fn default() -> Self {
        crate::rpc::sdkerrors::USER_DISCONNECTED.into()
    }
}

impl IntoUnknownError for PairDeleteRequest {
    fn unknown(&self) -> ResponseParamsError {
        ResponseParamsError::PairDelete(ErrorParams::unknown())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use super::super::tests::param_serde_test;
    use super::*;

    #[test]
    fn test_serde_pair_delete_request() -> Result<()> {
        let j = json! ({
            "code": 1,
            "message": "Error"
        });

        param_serde_test::<PairDeleteRequest>(&j.to_string())
    }
}
