mod log;

use {
    gloo_timers::future::TimeoutFuture,
    monedero_mesh::{
        domain::{
            namespaces::{ChainId, ChainType, Chains},
            ProjectId,
        },
        ClientSession, Dapp, Metadata, NoopSessionHandler, PairingManager, ReownBuilder,
    },
    tracing::{error, info},
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::spawn_local,
    web_sys::console,
};

async fn pair_manager(p: ProjectId) -> Option<PairingManager> {
    let builder = ReownBuilder::new(p);
    match builder.build().await {
        Err(e) => {
            let msg = format!("failed to create pairing manager {}", e);
            console::error_1(&msg.into());
            None
        }
        Ok(pair_manager) => Some(pair_manager),
    }
}

async fn dapp_init(mgr: PairingManager) -> Option<Dapp> {
    match Dapp::new(
        mgr,
        Metadata {
            name: "monedero-mesh".to_string(),
            description: "reown but for rust".to_string(),
            url: "https://github.com/dougEfresh".to_string(),
            icons: vec![],
            verify_url: None,
            redirect: None,
        },
    )
    .await
    {
        Ok(dapp) => Some(dapp),
        Err(e) => {
            let msg = format!("failed to create pairing manager {}", e);
            console::error_1(&msg.into());
            None
        }
    }
}

async fn propose(dapp: &Dapp) -> Option<ClientSession> {
    let chains = Chains::from([
        ChainId::Solana(ChainType::Dev),
        ChainId::EIP155(alloy_chains::Chain::sepolia()),
    ]);
    info!("purposing chains {chains}");
    let result = dapp.propose(NoopSessionHandler, &chains).await;
    match result {
        Err(e) => {
            error!("failed to propose dapp {}", e);
            None
        }
        Ok((pairing, fut, cached)) => {
            if cached {
                return Some(fut.await.unwrap());
            }
            info!("pairingUri {pairing}");
            match fut.await {
                Err(e) => {
                    error!("failed to finalize session {e}");
                    None
                }
                Ok(session) => Some(session),
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    log::init();
    let project_id = std::env::var("PROJECT_ID")
        .unwrap_or_else(|_| String::from("987f2292c12194ae69ddb6c52ceb1d62"));
    spawn_local(async move {
        let p = ProjectId::from(project_id);
        let manager = pair_manager(p).await;
        if manager.is_none() {
            return;
        }
        let dapp = dapp_init(manager.unwrap()).await;
        if dapp.is_none() {
            return;
        }
        let dapp = dapp.unwrap();
        let session = propose(&dapp).await;
        if session.is_none() {
            return;
        }
        let session = session.unwrap();
        let _ = session.ping().await;
        TimeoutFuture::new(2000).await;
        session.delete().await;
    })
}
