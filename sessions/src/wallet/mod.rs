use crate::rpc::{
    Controller, Metadata, RelayProtocol, ResponseParamsError, ResponseParamsSuccess,
    RpcResponsePayload, SdkErrors, SessionProposeRequest, SessionProposeResponse,
    SessionSettleRequest,
};
use crate::session::PendingSession;
use crate::{ClientSession, Pairing, PairingManager, ProposeFuture, Result, SessionHandler, SessionSettled, WalletProposalHandler};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, warn};
use xtra::Error;
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
    proposal_handler: Address<WalletProposalActor>,
    metadata: Metadata,
}

impl Display for Wallet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.metadata.name)
    }
}

impl Debug for Wallet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[wallet:{}]", self.metadata.name)
    }
}

impl Wallet {
    #[tracing::instrument(skip(request), level = "info")]
    async fn send_settlement(
        &self,
        request: SessionProposeRequest,
        public_key: String,
    ) -> Result<()> {
        let session_topic = self.manager.register_dapp_pk(request.proposer).await?;
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
                metadata: self.metadata.clone(),
            },
            namespaces: settled.clone(),
            expiry: future.timestamp(),
        };

        self.pending
            .settled(
                &self.manager,
                SessionSettled {
                    topic: session_topic,
                    namespaces: settled,
                    expiry: session_settlement.expiry,
                },
                Some(session_settlement),
            )
            .await?;
        Ok(())
    }
}

async fn send_settlement(wallet: Wallet, request: SessionProposeRequest, public_key: String) {
    // give time for dapp to get my public key and subscribe when in mock mode
    #[cfg(feature = "mock")]
    tokio::time::sleep(Duration::from_millis(1000)).await;

    if let Err(e) = wallet.send_settlement(request, public_key).await {
        warn!("failed to create ClientSession: '{e}'");
    }
}

/*
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
 */

impl Handler<SessionProposeRequest> for Wallet {
    type Return = RpcResponsePayload;

    async fn handle(
        &mut self,
        message: SessionProposeRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Return {
        let pk = self.manager.pair_key();
        if pk.is_none() {
            error!("no pairing key!");
            return RpcResponsePayload::Error(ResponseParamsError::SessionPropose(SdkErrors::UserRejected.into()))
        }
        let pk = pk.unwrap();
        if let Ok((accepted, response)) = self.proposal_handler.send(SessionProposePublicKey(String::from(&pk), message.clone())).await {
            if accepted {
                let wallet = self.clone();
                tokio::spawn(async move { send_settlement(wallet, message, pk).await });
            }
            return response
        }
        error!("failed sending verify to actor");
        RpcResponsePayload::Error(ResponseParamsError::SessionPropose(SdkErrors::UserRejected.into()))
    }
}


#[derive(Clone, Actor)]
struct WalletProposalActor {
    handler: Arc<Mutex<Box<dyn WalletProposalHandler>>>
}

impl WalletProposalActor {
    pub fn new<T: WalletProposalHandler>(handler: T) -> Self {
        Self {
            handler: Arc::new(Mutex::new(Box::new(handler)))
        }
    }
}

struct SessionProposePublicKey(pub String, pub SessionProposeRequest);

impl Handler<SessionProposePublicKey> for WalletProposalActor {
    type Return = (bool, RpcResponsePayload);

    async fn handle(
        &mut self,
        message: SessionProposePublicKey,
        _ctx: &mut Context<Self>) -> Self::Return {
        let l = self.handler.lock().await;
        l.verify_settlement(message.1, message.0).await
    }
}

impl Wallet {

    pub async fn new<T: WalletProposalHandler>(manager: PairingManager, handler: T) -> Result<Self> {
        let metadata = Metadata {
            name: "mock wallet".to_string(),
            description: "mocked wallet".to_string(),
            url: "https://example.com".to_string(),
            icons: vec![],
            verify_url: None,
            redirect: None,
        };
        let proposal_handler = xtra::spawn_tokio(WalletProposalActor::new(handler), Mailbox::unbounded());

        let me = Self {
            manager,
            pending: Arc::new(PendingSession::new()),
            metadata,
            proposal_handler
        };

        me.manager.actors().proposal().send(me.clone()).await?;
        Ok(me)
    }

    #[tracing::instrument(skip(handlers), level = "info")]
    pub async fn pair<T: SessionHandler>(
        &self,
        uri: String,
        handlers: T,
    ) -> Result<(Pairing, ProposeFuture)> {
        let pairing = Pairing::from_str(&uri)?;
        let rx = self.pending.add(pairing.topic.clone(), handlers);
        self.manager.set_pairing(pairing.clone()).await?;
        Ok((pairing, ProposeFuture::new(rx)))
    }
}
