use {
    crate::rpc::{ErrorParams, IntoUnknownError, ResponseParamsError},
    monedero_domain::namespaces::ChainId,
    serde::{Deserialize, Serialize},
    std::fmt::{Display, Formatter},
};

/// (wc_sessionRequest)[https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods#wc_sessionrequest]
use super::IrnMetadata;

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: 1108,
    ttl: 300,
    prompt: true,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: 1109,
    ttl: 300,
    prompt: false,
};

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequestMethod {
    pub method: monedero_domain::namespaces::Method,
    /// Opaque blockchain RPC parameters.
    ///
    /// Parsing is deferred to a higher level, blockchain RPC aware code.
    pub params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<u64>,
}

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionRequestRequest {
    pub request: RequestMethod,
    pub chain_id: ChainId,
}

impl Display for SessionRequestRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "chain: {} method: {}",
            self.chain_id, self.request.method
        )
    }
}

impl IntoUnknownError for SessionRequestRequest {
    fn unknown(&self) -> ResponseParamsError {
        ResponseParamsError::SessionRequest(ErrorParams::unknown())
    }
}

#[cfg(test)]
mod tests {
    use {
        super::{super::tests::param_serde_test, *},
        anyhow::Result,
    };

    #[test]
    fn test_serde_eth_sign_transaction() -> Result<()> {
        // https://specs.walletconnect.com/2.0/specs/clients/sign/
        // session-events#session_request
        let json = r#"
        {
            "request": {
                "method": "eth_signTransaction",
                "params": [
                    {
                        "from": "0x1456225dE90927193F7A171E64a600416f96f2C8",
                        "to": "0x1456225dE90927193F7A171E64a600416f96f2C8",
                        "data": "0x",
                        "nonce": "0x00",
                        "gasPrice": "0xa72c",
                        "gasLimit": "0x5208",
                        "value": "0x00"
                    }
                ]
            },
            "chainId": "eip155:5"
        }
        "#;

        param_serde_test::<SessionRequestRequest>(json)
    }
}
