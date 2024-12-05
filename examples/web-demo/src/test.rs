use monedero_domain::namespaces::{ChainId, ChainType, Chains};

use gloo_timers::future::TimeoutFuture;
use monedero_mesh::{
    auth_token, ClientSession, Dapp, Metadata, NoopSessionHandler, WalletConnectBuilder,
};
use tracing::{error, info};
use {monedero_mesh::PairingManager, monedero_relay::ProjectId, wasm_bindgen_test::*};

wasm_bindgen_test_configure!(run_in_browser);

async fn pair_manager(p: ProjectId) -> anyhow::Result<PairingManager> {
    let auth = auth_token("https://github.com/dougEfresh");
    let builder = WalletConnectBuilder::new(p, auth);
    Ok(builder.build().await?)
}

async fn dapp_init(mgr: PairingManager) -> anyhow::Result<Dapp> {
    let dapp = Dapp::new(
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
    .await?;
    Ok(dapp)
}

async fn propose(dapp: &Dapp) -> anyhow::Result<ClientSession> {
    let chains = Chains::from([
        ChainId::Solana(ChainType::Test),
        ChainId::EIP155(alloy_chains::Chain::sepolia()),
    ]);
    info!("purposing chains {chains}");
    let (pairing, fut, cached) = dapp.propose(NoopSessionHandler, &chains).await?;
    if cached {
        return Ok(fut.await?);
    }
    info!("pairingUri {pairing}");
    Ok(fut.await?)
}

#[wasm_bindgen_test]
async fn test_sessions() -> anyhow::Result<()> {
    let p = ProjectId::from("987f2292c12194ae69ddb6c52ceb1d62");
    let manager = pair_manager(p).await?;
    let dapp = dapp_init(manager).await?;
    let session = propose(&dapp).await?;
    let _ = session.ping().await;
    TimeoutFuture::new(2000).await;
    session.delete().await;
    Ok(())
}
