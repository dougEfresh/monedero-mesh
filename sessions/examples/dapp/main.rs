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
use walletconnect_sessions::rpc::{Metadata, ProposeNamespace, ProposeNamespaces};
use walletconnect_sessions::{ClientSession, Dapp, KvStorage, WalletConnectBuilder};
use walletconnect_sessions::{Pairing, ProjectId};

async fn propose(dapp: &Dapp) -> anyhow::Result<(Pairing, ClientSession)> {
    let required: Vec<alloy_chains::Chain> = vec![alloy_chains::Chain::sepolia()];
    let mut namespaces: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
    namespaces.insert(String::from("eip155"), required.into());
    let (p, rx) = dapp.propose(ProposeNamespaces(namespaces)).await?;
    println!("\n\n{pairing}\n\n");
    let session = rx.await??;
    Ok((p, session))
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
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        println!("session ping");
        if let Err(e) = session.ping().await {
            eprintln!("ping failed! {e}");
        }
    }
}

async fn dapp_test() -> anyhow::Result<()> {
    info!("starting sanity test");
    let auth = walletconnect_sessions::auth_token("https://github.com/dougEfresh");
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
