use {
    async_trait::async_trait,
    monedero_domain::{
        namespaces::{
            Account,
            Accounts,
            ChainId,
            Chains,
            EipMethod,
            Events,
            Method,
            Methods,
            Namespace,
            NamespaceName,
            Namespaces,
            SolanaMethod,
        },
        ProjectId,
    },
    monedero_mesh::{
        init_tracing,
        mock_connection_opts,
        rpc::{
            Metadata,
            RelayProtocol,
            ResponseParamsError,
            ResponseParamsSuccess,
            RpcResponsePayload,
            SessionProposeRequest,
            SessionProposeResponse,
        },
        Actors,
        Dapp,
        KvStorage,
        MockRelay,
        ReownBuilder,
        Result,
        SdkErrors,
        Wallet,
        WalletSettlementHandler,
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        time::Duration,
    },
};

//#[allow(dead_code)]
// pub static INIT: Once = Once::new();
//
#[allow(dead_code)]
pub struct TestStuff {
    pub dapp_actors: Actors,
    pub(crate) wallet_actors: Actors,
    pub(crate) dapp: Dapp,
    pub(crate) wallet: Wallet,
    pub relay: MockRelay,
}

pub async fn yield_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

pub struct WalletProposal {}

pub const SUPPORTED_ACCOUNT: &str = "0xBA5BA3955463ADcc7aa3E33bbdfb8A68e0933dD8";

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
            settled.insert(name.clone(), Namespace {
                accounts: Accounts(accounts),
                chains: Chains(namespace.chains.iter().cloned().collect()),
                methods: Methods(methods),
                events: Events::default(),
            });
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
                relay: RelayProtocol::default(),
                responder_public_key: pk,
            },
        ));
        (true, result)
    }
}

pub async fn init_test_components() -> anyhow::Result<TestStuff> {
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
