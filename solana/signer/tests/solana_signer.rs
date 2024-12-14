use {
    monedero_signer_solana::{
        domain::namespaces::{ChainId, ChainType},
        session::{init_tracing, mock_connection_opts, NoopSessionHandler, ProposeFuture, Wallet},
        Dapp, KvStorage, Metadata, MockWallet, ProjectId, ReownBuilder, ReownSigner, SolanaSession,
    },
    solana_pubkey::Pubkey,
    solana_sdk::{signer::Signer, transaction::Transaction},
    std::{str::FromStr, time::Duration},
    tokio::time::timeout,
    tracing::{error, info},
};

#[allow(dead_code)]
pub struct TestContext {
    pub dapp: Dapp,
    pub session: monedero_signer_solana::SolanaSession,
    pub wallet: MockWallet,
}

pub(crate) async fn yield_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

pub(crate) async fn init_test_components() -> anyhow::Result<TestContext> {
    init_tracing();
    let p = ProjectId::from("987f2292c12194ae69ddb6c52ceb1d62");
    let dapp_opts = mock_connection_opts(&p);
    let wallet_opts = mock_connection_opts(&p);
    let _ = monedero_mesh::MockRelay::start().await?;
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
    let sol_session = pair_dapp_wallet(&dapp, &wallet, mock_wallet.clone()).await?;
    Ok(TestContext {
        dapp,
        wallet: mock_wallet,
        session: sol_session,
    })
}

async fn await_wallet_pair(rx: ProposeFuture) {
    match timeout(Duration::from_secs(5), rx).await {
        Ok(s) => match s {
            Ok(_) => {
                info!("wallet got client session");
            }
            Err(e) => error!("wallet got error session: {e}"),
        },
        Err(e) => error!("timeout for wallet to recv client session from wallet: {e}"),
    }
}

async fn pair_dapp_wallet(
    dapp: &Dapp,
    wallet: &Wallet,
    mock_wallet: MockWallet,
) -> anyhow::Result<SolanaSession> {
    let (pairing, rx, _) = dapp
        .propose(NoopSessionHandler, &[ChainId::Solana(ChainType::Dev)])
        .await?;
    info!("got pairing topic {pairing}");
    let (_, wallet_rx) = wallet.pair(pairing.to_string(), mock_wallet).await?;
    tokio::spawn(async move { await_wallet_pair(wallet_rx).await });
    let session = timeout(Duration::from_secs(5), rx).await??;
    // let wallet get their ClientSession
    yield_ms(1000).await;
    let sol_session = SolanaSession::try_from(&session)?;

    Ok(sol_session)
}

fn transfer(signer: &ReownSigner, to: &Pubkey, lamports: u64) -> anyhow::Result<()> {
    let payer = &signer.pubkey();
    let instruction = solana_sdk::system_instruction::transfer(payer, to, lamports);
    let mut tx = Transaction::new_with_payer(&[instruction], Some(payer));
    // let blockhash = rpc.get_latest_blockhash().await?;
    let blockhash = solana_sdk::hash::Hash::default();
    tx.try_sign(&[signer], blockhash)?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_session() -> anyhow::Result<()> {
    let tc = init_test_components().await?;
    info!("settlement complete pk is {}", tc.session);
    let to = Pubkey::from_str("E4SfgGV2v9GLYsEkCQhrrnFbBcYmAiUZZbJ7swKGzZHJ")?;
    let signer = ReownSigner::new(tc.session.clone());
    transfer(&signer, &to, 1)?;
    let msg = format!("{}", chrono::Utc::now());
    let sig = tc.session.sign_message(&msg).await?;
    info!("signed message result {sig}");

    Ok(())
}
