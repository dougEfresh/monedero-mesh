use {
    gloo_timers::future::TimeoutFuture,
    monedero_mesh::{
        domain::{auth_token, ProjectId},
        Dapp,
        Metadata,
        PairingManager,
        WalletConnectBuilder,
    },
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::spawn_local,
    web_sys::console,
};

async fn pair_manager(p: ProjectId) -> Option<PairingManager> {
    let auth = auth_token("https://github.com/dougEfresh");
    let builder = WalletConnectBuilder::new(p, auth);
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
    match Dapp::new(mgr, Metadata {
        name: "monedero-mesh".to_string(),
        description: "reown but for rust".to_string(),
        url: "https://github.com/dougEfresh".to_string(),
        icons: vec![],
        verify_url: None,
        redirect: None,
    })
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

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    let project_id = env!["PROJECT_ID", "987f2292c12194ae69ddb6c52ceb1d62"];
    tracing_wasm::set_as_global_default();
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
        TimeoutFuture::new(2000).await;
    })
}
