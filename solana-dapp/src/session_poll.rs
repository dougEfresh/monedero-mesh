use crate::config::AppConfig;
use crate::message::UserEvent;
use crate::Msg;
use dashmap::DashMap;
use monedero_solana::monedero_mesh::{
    auth_token, Dapp, Metadata, NoopSessionHandler, Pairing, ProjectId, ProposeFuture, Topic,
    WalletConnectBuilder,
};
use monedero_solana::{SolanaSession, TokenTransferClient, WalletConnectSigner};
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tuirealm::listener::{ListenerResult, Poll};
use tuirealm::Event;

#[derive(Clone, PartialEq, Debug)]
pub struct DappContext {
    pub session: SolanaSession,
    pub signer: WalletConnectSigner,
}

impl PartialOrd for DappContext {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.session.pubkey().partial_cmp(&other.session.pubkey())
    }
}

impl Eq for DappContext {}

#[derive(Clone, PartialEq, Debug)]
enum SettlementState {
    None,
    Error(String),
    Settled(DappContext),
}

#[derive(Clone)]
pub struct SessionPoll {
    pub pairing: Pairing,
    // lazy lock
    session_state: Arc<DashMap<Topic, SettlementState>>,
}

impl SessionPoll {
    pub async fn init(config: AppConfig) -> anyhow::Result<Self> {
        let project = ProjectId::from("1760736b8b49aeb707b1a80099e51e58");
        let auth = auth_token("https://github.com/dougEfresh");
        let mgr = WalletConnectBuilder::new(project, auth).build().await?;
        let dapp = Dapp::new(
            mgr,
            Metadata {
                name: env!("CARGO_CRATE_NAME").to_string(),
                description: "solana dapp tui with walletconnect".to_string(),
                url: "https://github.com/dougeEfresh/monedero-mesh".to_string(),
                icons: vec![],
                verify_url: None,
                redirect: None,
            },
        )
        .await?;

        let (pairing, propose_fut, _) = dapp.propose(NoopSessionHandler, &config.chains).await?;
        tracing::info!("{pairing}");
        let state = DashMap::<Topic, SettlementState>::new();
        let me = Self {
            //dapp,
            pairing,
            session_state: Arc::new(state),
        };
        let cloned = me.clone();
        tokio::spawn(async move { finalize_state(cloned, propose_fut).await });
        Ok(me)
    }
}

impl Poll<UserEvent> for SessionPoll {
    fn poll(&mut self) -> ListenerResult<Option<Event<UserEvent>>> {
        tracing::debug!("poll request");
        if self.session_state.is_empty() {
            return Ok(None);
        }
        if let Some((t, state)) = self.session_state.remove(&self.pairing.topic) {
            return match state {
                SettlementState::None => {
                    // nothing happend yet, put back in map
                    self.session_state.insert(t, SettlementState::None);
                    Ok(None)
                }
                SettlementState::Error(e) => {
                    Ok(Some(Event::User(UserEvent::SettledError(e.to_string()))))
                }
                SettlementState::Settled(dapp) => {
                    tracing::debug!("settled returning to tuireal");
                    Ok(Some(Event::User(UserEvent::Settled(dapp))))
                }
            };
        }
        tracing::error!("This should never happen");
        Ok(None)
    }
}

async fn finalize_state(poller: SessionPoll, fut: ProposeFuture) {
    let session_result = fut.await;
    let p = poller.pairing.topic.clone();
    match session_result {
        Ok(ref s) => {
            let result = SolanaSession::try_from(s);
            match result {
                Ok(s) => {
                    let dapp = DappContext {
                        session: s.clone(),
                        signer: WalletConnectSigner::new(s),
                    };
                    poller
                        .session_state
                        .insert(p, SettlementState::Settled(dapp));
                }
                Err(e) => {
                    poller
                        .session_state
                        .insert(p, SettlementState::Error(e.to_string()));
                }
            };
        }
        Err(e) => {
            poller
                .session_state
                .insert(p, SettlementState::Error(e.to_string()));
        }
    };
}
