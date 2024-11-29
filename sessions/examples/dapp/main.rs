mod app;
mod config;
mod event_reader;
mod input;
mod log;
mod msg;
mod runner;
mod ui;

use {
    crate::log::initialize_logging,
    monedero_mesh::{
        self,
        ClientSession,
        Dapp,
        KvStorage,
        Metadata,
        NoopSessionHandler,
        Pairing,
        ProjectId,
        WalletConnectBuilder,
    },
    monedero_namespaces::{ChainId, ChainType, Chains},
    std::{
        collections::BTreeMap,
        panic::{set_hook, take_hook},
        time::Duration,
    },
    tokio::{select, signal},
    tracing::info,
};

async fn propose(dapp: &Dapp) -> anyhow::Result<(Pairing, ClientSession)> {
    let chains = Chains::from([
        ChainId::Solana(ChainType::Test),
        ChainId::EIP155(alloy_chains::Chain::sepolia()),
    ]);
    let (p, rx, restored) = dapp.propose(NoopSessionHandler, &chains).await?;
    if !restored {
        println!("\n\n{p}\n\n");
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
    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
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
    let runner = runner::Runner {};
    runner.run()
    // dapp_test().await
}
