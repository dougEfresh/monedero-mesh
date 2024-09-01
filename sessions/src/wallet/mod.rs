use crate::actors::{RegisterComponent, RegisterPairing, SessionSettled};
use crate::pairing_uri::Params;
use crate::rpc::{
    Controller, Metadata, RelayProtocol, RequestParams, ResponseParamsError, ResponseParamsSuccess,
    RpcResponsePayload, SdkErrors, SessionProposeRequest, SessionProposeResponse,
    SessionSettleRequest,
};
use crate::session::PendingSession;
use crate::Error::NoPairingTopic;
use crate::{
    ClientSession, Pairing, PairingManager, ProposeFuture, Result, SessionHandlers, Topic,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{info, warn};
use walletconnect_namespaces::{
    Account, Accounts, ChainId, Chains, EipMethod, Events, Method, Methods, Namespace,
    NamespaceName, Namespaces, SolanaMethod,
};
use xtra::prelude::*;

const SUPPORTED_ACCOUNT: &str = "0xBA5BA3955463ADcc7aa3E33bbdfb8A68e0933dD8";

#[derive(Clone, xtra::Actor)]
pub struct Wallet {
    manager: PairingManager,
    pending: Arc<PendingSession>,
}

impl Wallet {
    async fn send_settlement(
        &self,
        request: SessionProposeRequest,
        public_key: String,
    ) -> Result<()> {
        let actors = self.manager.actors();
        let register = RegisterPairing {
            pairing: self.manager.pairing().ok_or(NoPairingTopic)?,
            mgr: self.manager.clone(),
            component: RegisterComponent::WalletDappPublicKey(
                self.clone(),
                request.proposer.clone(),
            ),
        };
        let session_topic = actors
            .register_pairing(register)
            .await?
            .ok_or(NoPairingTopic)?;
        let now = chrono::Utc::now();
        let future = now + chrono::Duration::hours(24);
        let mut settled: Namespaces = Namespaces(BTreeMap::new());
        for (name, namespace) in request.required_namespaces.iter() {
            let accounts: BTreeSet<Account> = namespace
                .chains
                .iter()
                .map(|c| Account {
                    address: String::from(SUPPORTED_ACCOUNT),
                    chain: c.clone(),
                })
                .collect();

            let methods = match name {
                NamespaceName::EIP155 => EipMethod::defaults(),
                NamespaceName::Solana => SolanaMethod::defaults(),
                NamespaceName::Other(_) => BTreeSet::from([Method::Other("unknown".to_owned())]),
            };
            settled.insert(
                name.clone(),
                Namespace {
                    accounts: Accounts(accounts),
                    chains: Chains(namespace.chains.iter().cloned().collect()),
                    methods: Methods(methods),
                    events: Events::default(),
                },
            );
        }

        let session_settlement = SessionSettleRequest {
            relay: RelayProtocol::default(),
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
            namespaces: settled,
            expiry: future.timestamp(),
        };

        self.pending
            .settled(
                &self.manager,
                SessionSettled(session_topic.clone(), session_settlement.clone()),
                true,
            )
            .await?;
        Ok(())
    }
}

async fn send_settlement(wallet: Wallet, request: SessionProposeRequest, public_key: String) {
    // give time for dapp to get my public key and subscribe
    tokio::time::sleep(Duration::from_millis(1000)).await;
    if let Err(e) = wallet.send_settlement(request, public_key).await {
        warn!("failed to create ClientSession: '{e}'")
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
    let reject_chain = ChainId::EIP155(alloy_chains::Chain::goerli());
    if let Some(ns) = proposal.required_namespaces.0.get(&NamespaceName::EIP155) {
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
                relay: RelayProtocol::default(),
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
        ctx: &mut Context<Self>,
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
        Self {
            manager,
            pending: Arc::new(PendingSession::new()),
        }
    }

    pub async fn pair<T: SessionHandlers>(
        &self,
        uri: String,
        handlers: T,
    ) -> Result<(Pairing, ProposeFuture<Result<ClientSession>>)> {
        let pairing = Pairing::from_str(&uri)?;
        let register = RegisterPairing {
            pairing: pairing.clone(),
            mgr: self.manager.clone(),
            component: RegisterComponent::WalletPairTopic(self.clone()),
        };
        let rx = self.pending.add(pairing.topic.clone(), handlers);
        self.manager.actors().register_pairing(register).await?;
        Ok((pairing, ProposeFuture::new(rx)))
    }
}
