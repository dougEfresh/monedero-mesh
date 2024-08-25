use crate::actors::{SendRequest, SessionSettled};
use crate::rpc::{
    Controller, Metadata, Proposer, RequestParams, Response, ResponseParamsError,
    ResponseParamsSuccess, RpcResponsePayload, SdkErrors, SessionProposeRequest,
    SessionProposeResponse, SessionSettleRequest, SettleNamespace, SettleNamespaces,
};
use crate::{rpc, Pairing, PairingManager, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::ops::Deref;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::oneshot::error::RecvError;
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use tracing::{info, warn};
use xtra::prelude::*;
const SUPPORTED_ACCOUNT: &str = "0xBA5BA3955463ADcc7aa3E33bbdfb8A68e0933dD8";

#[derive(Clone, xtra::Actor)]
pub struct Wallet {
    manager: PairingManager,
}

async fn send_settlement(wallet: Wallet, request: SessionProposeRequest, public_key: String) {
    // give time for dapp to get my public key and subscribe
    tokio::time::sleep(Duration::from_millis(1000)).await;
    info!("sending settlement {}", request.proposer.public_key);
    let actors = wallet.manager.actors();
    let session_topic = actors
        .register_dapp_pk(wallet.clone(), request.proposer)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    let future = now + chrono::Duration::hours(24);
    let mut settled: BTreeMap<String, SettleNamespace> = BTreeMap::new();
    if !request.required_namespaces.deref().contains_key("eip155") {
        warn!("not sending settlement due to no eip155 chains");
        return;
    }
    let mut namespaces = request.required_namespaces.0;
    let ns = namespaces.remove("eip155").unwrap();
    let eip = SettleNamespace {
        accounts: ns
            .chains
            .iter()
            .map(|c| format!("{}:{}", c, SUPPORTED_ACCOUNT))
            .collect(),
        methods: BTreeSet::from(ns.methods),
        events: BTreeSet::from(ns.events),
        extensions: None,
    };
    settled.insert(String::from("eip155"), eip);
    let session_settlement = SessionSettleRequest {
        relay: Default::default(),
        controller: Controller {
            public_key,
            metadata: Metadata {
                name: "mock wallet".to_string(),
                description: "mocked wallet".to_string(),
                url: "https://example.com".to_string(),
                icons: vec![],
                verify_url: None,
                redirect: None,
            },
        },
        namespaces: SettleNamespaces(settled),
        expiry: future.timestamp() as u64,
    };
    match actors
        .register_settlement(
            wallet.manager.topic_transport(),
            SessionSettled(session_topic, session_settlement.clone()),
        )
        .await
    {
        Err(e) => warn!("failed to register settlement for wallet {e}"),
        Ok(client_session) => {
            let request = RequestParams::SessionSettle(session_settlement);
            if let Err(e) = client_session.publish_request::<bool>(request).await {
                warn!("failed to send session settlement: {e}");
            }
        }
    }
}

fn verify_settlement(
    proposal: &SessionProposeRequest,
    pk: Option<String>,
) -> (bool, String, RpcResponsePayload) {
    if pk.is_none() {
        return (
            false,
            String::new(),
            RpcResponsePayload::Error(ResponseParamsError::SessionPropose(
                SdkErrors::UserRejected.into(),
            )),
        );
    }
    let pk = pk.unwrap();
    let reject_chain = format!("eip155:{}", alloy_chains::Chain::goerli().id());
    if let Some(ns) = proposal.required_namespaces.0.get("eip155") {
        if ns.chains.contains(&reject_chain) {
            return (
                false,
                String::new(),
                RpcResponsePayload::Error(ResponseParamsError::SessionPropose(
                    SdkErrors::UnsupportedChains.into(),
                )),
            );
        }
    }
    (
        true,
        String::from(&pk),
        RpcResponsePayload::Success(ResponseParamsSuccess::SessionPropose(
            SessionProposeResponse {
                relay: Default::default(),
                responder_public_key: pk,
            },
        )),
    )
}

impl Handler<SessionProposeRequest> for Wallet {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        message: SessionProposeRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        let (accepted, my_pk, response) = verify_settlement(&message, self.manager.pair_key());
        if accepted {
            let wallet = self.clone();
            tokio::spawn(async move { send_settlement(wallet, message, my_pk).await });
        }
        response
    }
}

impl Wallet {
    pub fn new(manager: PairingManager) -> Self {
        Self { manager }
    }

    pub async fn pair(&self, uri: String) -> Result<Pairing> {
        let pairing = Pairing::from_str(&uri)?;
        self.manager
            .actors()
            .register_wallet_pairing(self.clone(), pairing.clone())
            .await?;
        Ok(pairing)
    }
}
