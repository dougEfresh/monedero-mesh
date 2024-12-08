use {
    monedero_solana::domain::namespaces::{ChainId, ChainType},
    monedero_solana::session::{
        init_tracing, mock_connection_opts, MockRelay, NoopSessionHandler, ProposeFuture, Wallet,
    },
    monedero_solana::{Dapp, KvStorage, Metadata, ProjectId, ReownBuilder, SolanaSession},
    solana_pubkey::Pubkey,
    solana_signature::Signature,
    std::{str::FromStr, time::Duration},
    tokio::time::timeout,
    tracing::{error, info},
};

mod mock_wallet;
use mock_wallet::*;

fn explorer(sig: &Signature) {
    info!("https://solscan.io/tx/{sig}?cluster=devnet");
}

pub(crate) async fn yield_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

pub(crate) async fn init_test_components() -> anyhow::Result<(Dapp, Wallet, MockWallet, MockRelay)>
{
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

    let md = Metadata {
        name: "mock-dapp".to_string(),
        ..Default::default()
    };

    let mock_wallet = MockWallet {};
    let dapp = Dapp::new(dapp_manager, md).await?;
    let wallet = Wallet::new(wallet_manager, mock_wallet.clone()).await?;
    //dotenvy::dotenv()?;
    //let url = std::env::var(ChainId::Solana(ChainType::Dev).to_string())
    //    .ok()
    //    .unwrap_or(String::from("https://api.devnet.solana.com"));
    //info!("using url {url}");
    //let rpc_client = Arc::new(RpcClient::new(url));
    Ok((dapp, wallet, mock_wallet, relay))
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

async fn pair_dapp_wallet() -> anyhow::Result<(SolanaSession, MockWallet)> {
    let (dapp, wallet, mock_wallet, _) = init_test_components().await?;
    let (pairing, rx, _) = dapp
        .propose(NoopSessionHandler, &[ChainId::Solana(ChainType::Dev)])
        .await?;
    info!("got pairing topic {pairing}");
    let (_, wallet_rx) = wallet
        .pair(pairing.to_string(), mock_wallet.clone())
        .await?;
    tokio::spawn(async move { await_wallet_pair(wallet_rx).await });
    let session = timeout(Duration::from_secs(5), rx).await??;
    // let wallet get their ClientSession
    yield_ms(1000).await;
    let sol_session = SolanaSession::try_from(&session)?;
    Ok((sol_session, mock_wallet))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_session() -> anyhow::Result<()> {
    let (session, _) = pair_dapp_wallet().await?;
    info!("settlement complete pk is {session}");
    let to = Pubkey::from_str("E4SfgGV2v9GLYsEkCQhrrnFbBcYmAiUZZbJ7swKGzZHJ")?;
    let amount = 1;
    Ok(())
}
