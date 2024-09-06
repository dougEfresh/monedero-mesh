use anyhow::format_err;
use assert_matches::assert_matches;
use async_trait::async_trait;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Once;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
use walletconnect_namespaces::{
    Account, Accounts, AlloyChain, ChainId, ChainType, Chains, EipMethod, Events, Method, Methods,
    Namespace, NamespaceName, Namespaces, SolanaMethod,
};
use walletconnect_relay::{auth_token, ConnectionCategory, ConnectionOptions, ConnectionPair};
use walletconnect_session_solana::Solana;
use walletconnect_sessions::crypto::CipherError;
use walletconnect_sessions::rpc::{
    Metadata, ResponseParamsError, ResponseParamsSuccess, RpcResponsePayload,
    SessionProposeRequest, SessionProposeResponse,
};
use walletconnect_sessions::{
    Actors, ClientSession, Dapp, NoopSessionHandler, ProjectId, ProposeFuture,
    RegisteredComponents, SdkErrors, Wallet, WalletConnectBuilder, WalletProposalHandler,
};
use walletconnect_sessions::{Result, Topic};

#[allow(dead_code)]
static INIT: Once = Once::new();

pub(crate) async fn yield_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

struct WalletProposal {}

const SUPPORTED_ACCOUNT: &str = "215r9xfTFVYcE9g3fAUGowauM84egyUvFCbSo3LKNaep";

#[async_trait]
impl WalletProposalHandler for WalletProposal {
    async fn settlement(&self, proposal: SessionProposeRequest) -> Result<Namespaces> {
        let mut settled: Namespaces = Namespaces(BTreeMap::new());
        for (name, namespace) in proposal.required_namespaces.iter() {
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
        Ok(settled)
    }
}

pub(crate) async fn init_test_components() -> anyhow::Result<(Dapp, Wallet)> {
    init_tracing();
    let shared_id = Topic::generate();
    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
    let auth = auth_token("https://github.com/dougEfresh");
    let dapp_id = ConnectionPair(shared_id.clone(), ConnectionCategory::Dapp);
    let wallet_id = ConnectionPair(shared_id.clone(), ConnectionCategory::Wallet);
    let dapp_opts = ConnectionOptions::new(p.clone(), auth.clone(), dapp_id);
    let wallet_opts = ConnectionOptions::new(p.clone(), auth.clone(), wallet_id);
    let dapp_manager = WalletConnectBuilder::new(p.clone(), auth.clone())
        .connect_opts(dapp_opts)
        .build()
        .await?;
    let wallet_manager = WalletConnectBuilder::new(p, auth)
        .connect_opts(wallet_opts)
        .build()
        .await?;
    let md = Metadata {
        name: "mock-dapp".to_string(),
        ..Default::default()
    };
    let dapp = Dapp::new(dapp_manager, md).await?;
    let wallet = Wallet::new(wallet_manager, WalletProposal {}).await?;
    yield_ms(500).await;
    Ok((dapp, wallet))
}

pub(crate) fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_target(true)
            .with_level(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    });
}

async fn await_wallet_pair(rx: ProposeFuture) {
    match timeout(Duration::from_secs(5), rx).await {
        Ok(s) => match s {
            Ok(_) => {
                info!("wallet got client session")
            }
            Err(e) => error!("wallet got error session: {e}"),
        },
        Err(e) => error!("timeout for wallet to recv client session from wallet: {e}"),
    }
}

async fn pair_dapp_wallet() -> anyhow::Result<ClientSession> {
    let (dapp, wallet) = init_test_components().await?;
    let (pairing, rx, _) = dapp
        .propose(NoopSessionHandler, &[ChainId::Solana(ChainType::Test)])
        .await?;
    info!("got pairing topic {pairing}");
    let (_, wallet_rx) = wallet.pair(pairing.to_string(), NoopSessionHandler).await?;
    tokio::spawn(async move { await_wallet_pair(wallet_rx).await });
    let session = timeout(Duration::from_secs(5), rx).await??;
    // let wallet get their ClientSession
    yield_ms(1000).await;
    Ok(session)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_session() -> anyhow::Result<()> {
    let session = pair_dapp_wallet().await?;
    info!("settlement complete");
    let sol_session = Solana::try_from(&session)?;
    Ok(())
}
