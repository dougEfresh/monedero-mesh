mod app;
mod config;
mod event_reader;
mod input;
mod log;
mod msg;
mod runner;
mod ui;

use sessions;
use std::collections::BTreeMap;
use tracing::{error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

use crate::log::initialize_logging;
use sessions::rpc::{ProposeNamespace, ProposeNamespaces};
use sessions::WalletConnectBuilder;
use sessions::{KvStorage, PairingManager, RELAY_ADDRESS};
use std::time::Duration;
use tokio::{select, signal};
use walletconnect_sdk::rpc::auth::ed25519_dalek::SigningKey;
use walletconnect_sdk::rpc::auth::AuthToken;
use walletconnect_sdk::rpc::domain::ProjectId;

async fn do_dapp_stuff(pairing_mgr: PairingManager) {
    info!("Running dapp - hit control-c to terminate");
    let required: Vec<alloy_chains::Chain> = vec![alloy_chains::Chain::sepolia()];
    let mut namespaces: BTreeMap<String, ProposeNamespace> = BTreeMap::new();
    namespaces.insert(String::from("eip155"), required.into());
    let result = pairing_mgr.propose(ProposeNamespaces(namespaces)).await;

    let (pairing, rx) = match result {
        Ok((p, r)) => (p, r),
        Err(e) => {
            error!("failed to get pairing {e}");
            return;
        }
    };
    println!("\n\n\n{pairing}\n\n\n");

    let session = match rx.await {
        Ok(s) => match s {
            Ok(sess) => sess,
            Err(e) => {
                error!("session settlement failed {e}");
                return;
            }
        },
        Err(e) => {
            error!("crap {e}");
            return;
        }
    };
    info!("settled {:#?}", session.namespaces());
    tokio::time::sleep(Duration::from_secs(10)).await;
}

async fn dapp() -> anyhow::Result<()> {
    info!("starting sanity test");
    let key = SigningKey::generate(&mut rand::thread_rng());
    let auth = AuthToken::new("http://example.com")
        .aud(RELAY_ADDRESS)
        .ttl(Duration::from_secs(60 * 60))
        .as_jwt(&key)?;

    let p = ProjectId::from("9d5b20b16777cc49100cf9df3649bd24");
    let builder = WalletConnectBuilder::new(p, auth);
    let store = KvStorage::mem();
    let builder = builder.store(store);
    let pairing_mgr = builder.build().await?;
    //pairing_mgr.socket_open().await?;
    tokio::spawn(do_dapp_stuff(pairing_mgr.clone()));

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
}
