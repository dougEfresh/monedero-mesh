use {
    anyhow::format_err,
    assert_matches::assert_matches,
    async_trait::async_trait,
    monedero_domain::{
        namespaces::{
            Account, Accounts, AlloyChain, ChainId, ChainType, Chains, EipMethod, Events, Method,
            Methods, Namespace, NamespaceName, Namespaces, SolanaMethod,
        },
        ProjectId, Topic,
    },
    monedero_mesh::{
        auth_token, default_connection_opts, mock_connection_opts,
        rpc::{
            Metadata, ResponseParamsError, ResponseParamsSuccess, RpcResponsePayload,
            SessionProposeRequest, SessionProposeResponse,
        },
        Actors, ClientSession, Dapp, KvStorage, MockRelay, NoopSessionHandler, ProposeFuture,
        RegisteredComponents, ReownBuilder, Result, SdkErrors, Wallet, WalletSettlementHandler,
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        sync::Once,
        time::Duration,
    },
    tokio::time::timeout,
    tracing::{error, info},
    tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
};

#[allow(dead_code)]
static INIT: Once = Once::new();

pub(crate) struct TestStuff {
    pub(crate) dapp_actors: Actors,
    pub(crate) wallet_actors: Actors,
    pub(crate) dapp: Dapp,
    pub(crate) wallet: Wallet,
    relay: MockRelay,
}

pub(crate) async fn yield_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

struct WalletProposal {}

const SUPPORTED_ACCOUNT: &str = "0xBA5BA3955463ADcc7aa3E33bbdfb8A68e0933dD8";

#[async_trait]
impl WalletSettlementHandler for WalletProposal {
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

    async fn verify_settlement(
        &self,
        proposal: SessionProposeRequest,
        pk: String,
    ) -> (bool, RpcResponsePayload) {
        let reject_chain = ChainId::EIP155(alloy_chains::Chain::goerli());
        if let Some(ns) = proposal.required_namespaces.0.get(&NamespaceName::EIP155) {
            if ns.chains.contains(&reject_chain) {
                return (
                    false,
                    RpcResponsePayload::Error(ResponseParamsError::SessionPropose(
                        SdkErrors::UnsupportedChains.into(),
                    )),
                );
            }
        }
        let result = RpcResponsePayload::Success(ResponseParamsSuccess::SessionPropose(
            SessionProposeResponse {
                relay: Default::default(),
                responder_public_key: pk,
            },
        ));
        (true, result)
    }
}

pub(crate) async fn init_test_components() -> anyhow::Result<TestStuff> {
    init_tracing();
    let p = ProjectId::from("987f2292c12194ae69ddb6c52ceb1d62");
    let dapp_opts = mock_connection_opts(&p);
    let wallet_opts = mock_connection_opts(&p);
    let relay = monedero_mesh::MockRelay::start().await?;
    let dapp_manager = ReownBuilder::new(p.clone())
        .connect_opts(dapp_opts)
        .store(KvStorage::mem())
        .build()
        .await?;
    let wallet_manager = ReownBuilder::new(p)
        .connect_opts(wallet_opts)
        .store(KvStorage::mem())
        .build()
        .await?;
    let dapp_actors = dapp_manager.actors();
    let wallet_actors = wallet_manager.actors();
    let md = Metadata {
        name: "mock-dapp".to_string(),
        ..Default::default()
    };
    let dapp = Dapp::new(dapp_manager, md).await?;
    let wallet = Wallet::new(wallet_manager, WalletProposal {}).await?;
    yield_ms(500).await;
    let t = TestStuff {
        dapp_actors: dapp_actors.clone(),
        wallet_actors: wallet_actors.clone(),
        dapp,
        wallet,
        relay,
    };
    Ok(t)
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

async fn pair_dapp_wallet() -> anyhow::Result<(TestStuff, ClientSession)> {
    let t = init_test_components().await?;
    let dapp = t.dapp.clone();
    let wallet = t.wallet.clone();
    let (pairing, rx, _) = dapp
        .propose(
            NoopSessionHandler,
            &[
                ChainId::EIP155(alloy_chains::Chain::holesky()),
                ChainId::EIP155(alloy_chains::Chain::sepolia()),
                ChainId::Solana(ChainType::Dev),
            ],
        )
        .await?;
    info!("got pairing topic {pairing}");
    let (_, wallet_rx) = wallet.pair(pairing.to_string(), NoopSessionHandler).await?;
    tokio::spawn(async move { await_wallet_pair(wallet_rx).await });
    let session = timeout(Duration::from_secs(5), rx).await??;
    // let wallet get their ClientSession
    yield_ms(1000).await;
    Ok((t, session))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_dapp_settlement() -> anyhow::Result<()> {
    let (test, session) = pair_dapp_wallet().await?;
    info!("settlement complete");
    assert!(session.namespaces().contains_key(&NamespaceName::Solana));
    assert!(session.ping().await?);
    assert!(session.delete().await);
    let components = test
        .dapp_actors
        .session()
        .send(RegisteredComponents)
        .await?;
    assert_eq!(0, components);
    assert_matches!(
        session.ping().await,
        Err(monedero_mesh::Error::NoClientSession(_))
    );
    yield_ms(500).await;
    // propose again should repair
    let original_pairing = test.dapp.pairing().ok_or(format_err!("no pairing!"))?;
    let (new_pairing, rx, restored) = test
        .dapp
        .propose(
            NoopSessionHandler,
            &[ChainId::EIP155(AlloyChain::sepolia())],
        )
        .await?;
    assert!(!restored);
    assert_ne!(original_pairing.topic, new_pairing.topic);

    let (wallet_pairing, _) = test
        .wallet
        .pair(new_pairing.to_string(), NoopSessionHandler)
        .await?;
    assert_eq!(wallet_pairing.topic, new_pairing.topic);
    let session = timeout(Duration::from_secs(5), rx).await??;
    yield_ms(1000).await;
    let components = test
        .dapp_actors
        .session()
        .send(RegisteredComponents)
        .await?;
    assert_eq!(1, components);
    assert!(session.ping().await?);
    assert!(session.delete().await);
    // let's wait and see if any random background error show up
    yield_ms(5000).await;
    Ok(())
}
