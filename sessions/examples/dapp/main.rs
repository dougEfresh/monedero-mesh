mod log;
use {
    crate::log::initialize_logging,
    copypasta::{ClipboardContext, ClipboardProvider},
    monedero_domain::{
        namespaces::{ChainId, ChainType, Chains},
        Pairing,
        ProjectId,
    },
    monedero_mesh::{
        self,
        ClientSession,
        Dapp,
        Metadata,
        NoopSessionHandler,
        WalletConnectBuilder,
    },
    monedero_store::KvStorage,
    std::time::Duration,
    tokio::{select, signal},
    tracing::info,
};

async fn propose(dapp: &Dapp) -> anyhow::Result<(Pairing, ClientSession)> {
    let chains = Chains::from([
        ChainId::Solana(ChainType::Dev),
        ChainId::EIP155(alloy_chains::Chain::sepolia()),
    ]);
    info!("purposing chains {chains}");
    let (p, rx, restored) = dapp.propose(NoopSessionHandler, &chains).await?;
    let mut ctx = ClipboardContext::new().expect("Failed to open clipboard");
    ctx.set_contents(p.to_string())
        .expect("Failed to set clipboard");
    if !restored {
        qr2term::print_qr(&p.to_string())?;
        eprintln!("\n\n{p}\n\n");
    }
    let session = rx.await?;
    Ok((p, session))
}

async fn pair_ping(dapp: Dapp) {
    loop {
        println!("pair ping");
        if let Err(e) = dapp.pair_ping().await {
            eprintln!("pair ping failed! {e}");
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

async fn do_dapp_stuff(dapp: Dapp) {
    info!("Running dapp - hit control-c to terminate");
    let session = match propose(&dapp).await {
        Err(e) => {
            tracing::error!("failed to get session! {e}");
            return;
        }
        Ok((_, s)) => s,
    };
    info!("settled {:#?}", session.namespaces());
    let pinger = dapp.clone();
    tokio::spawn(pair_ping(pinger));
    loop {
        println!("session ping");
        if let Err(e) = session.ping().await {
            eprintln!("session ping failed! {e}");
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

async fn dapp_test() -> anyhow::Result<()> {
    info!("starting sanity test");
    let auth = monedero_relay::auth_token("https://github.com/dougEfresh");
    let p = ProjectId::from("987f2292c12194ae69ddb6c52ceb1d62");
    let store = KvStorage::file(None)?;
    let builder = WalletConnectBuilder::new(p, auth);
    let builder = builder.store(store);
    let pairing_mgr = builder.build().await?;
    let dapp = Dapp::new(pairing_mgr.clone(), Metadata {
        name: "wc-sessions-sdk".to_string(),
        description: "walletconnect sessions for rust".to_string(),
        url: "https://github.com/dougEfresh".to_string(),
        icons: vec![],
        verify_url: None,
        redirect: None,
    })
    .await?;
    tokio::spawn(do_dapp_stuff(dapp));

    let ctrl_c = signal::ctrl_c();
    let mut term = signal::unix::signal(signal::unix::SignalKind::terminate())?;

    select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = term.recv() => {
            info!("Received SIGTERM, shutting down...");
        }
    }
    pairing_mgr.shutdown().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_logging()?;
    dapp_test().await
}
