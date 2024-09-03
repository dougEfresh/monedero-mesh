mod app;
mod config;
mod event_reader;
mod input;
mod log;
mod msg;
mod runner;
mod ui;

use std::collections::BTreeMap;
use tracing::info;
use walletconnect_sessions;

use crate::log::initialize_logging;
use std::time::Duration;
use tokio::{select, signal};
use walletconnect_namespaces::{ChainId, ChainType, Chains};
use walletconnect_sessions::{Metadata, Pairing, ProjectId, ClientSession, Dapp, KvStorage, WalletConnectBuilder, NoopSessionHandler};

async fn propose(dapp: &Dapp) -> anyhow::Result<(Pairing, ClientSession)> {
    let chains = Chains::from([ChainId::Solana(ChainType::Test), ChainId::EIP155(alloy_chains::Chain::sepolia())]);
    let (p, rx) = dapp.propose(NoopSessionHandler, &chains).await?;
    println!("\n\n{p}\n\n");
    let session = rx.await??;
    Ok((p, session))
}

async fn pair_ping(dapp: Dapp) {
    loop {
        println!("pair ping");
        if let Err(e) = dapp.pair_ping().await {
            eprintln!("ping failed! {e}");
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
            eprintln!("ping failed! {e}");
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

async fn dapp_test() -> anyhow::Result<()> {
    info!("starting sanity test");
    let auth = walletconnect_relay::auth_token("https://github.com/dougEfresh");
    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
    let store = KvStorage::file(None)?;
    let builder = WalletConnectBuilder::new(p, auth);
    let builder = builder.store(store);
    let pairing_mgr = builder.build().await?;
    let dapp = Dapp::new(
        pairing_mgr.clone(),
        Metadata {
            name: "walletconnect-sessions-sdk".to_string(),
            description: "walletconnect sessions for rust".to_string(),
            url: "https://github.com/dougEfresh".to_string(),
            icons: vec![],
            verify_url: None,
            redirect: None,
        },
    );
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
    /*
    let runner = runner::Runner {};
    runner.run()
     */
    dapp_test().await
}
