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

mod test_utils;
use test_utils::*;

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
