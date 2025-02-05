//! (wc_sessionUpdate)[https://specs.walletconnect.com/2.0/specs/clients/sign/rpc-methods#wc_sessionupdate]

use {
    super::IrnMetadata,
    monedero_domain::namespaces::Namespaces,
    serde::{Deserialize, Serialize},
};

pub(super) const IRN_REQUEST_METADATA: IrnMetadata = IrnMetadata {
    tag: 1104,
    ttl: 86400,
    prompt: false,
};

pub(super) const IRN_RESPONSE_METADATA: IrnMetadata = IrnMetadata {
    tag: 1105,
    ttl: 86400,
    prompt: false,
};

#[derive(Debug, Serialize, PartialEq, Eq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionUpdateRequest {
    pub namespaces: Namespaces,
}

#[cfg(test)]
mod tests {
    use {
        super::{super::tests::param_serde_test, *},
        anyhow::Result,
    };

    #[test]
    fn test_serde_session_update_request() -> Result<()> {
        // https://specs.walletconnect.com/2.0/specs/clients/sign/
        // session-events#session_update
        let json = r#"
        {
            "namespaces": {
                "eip155": {
                    "accounts": [
                        "eip155:137:0x1456225dE90927193F7A171E64a600416f96f2C8",
                        "eip155:5:0x1456225dE90927193F7A171E64a600416f96f2C8"
                    ],
                    "methods": [
                        "eth_sendTransaction",
                        "eth_sign",
                        "eth_signTransaction",
                        "eth_signTypedData",
                        "personal_sign"
                    ],
                    "events": [
                        "accountsChanged",
                        "chainChanged"
                    ]
                }
            }
        }
        "#;

        param_serde_test::<SessionUpdateRequest>(json)
    }
}
