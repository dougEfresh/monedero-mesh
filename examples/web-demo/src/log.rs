use {
    tracing::Level,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt},
    wasm_tracing::{WASMLayer, WASMLayerConfigBuilder},
};

pub fn init() {
    console_error_panic_hook::set_once();
    let config = WASMLayerConfigBuilder::new()
        .set_max_level(Level::DEBUG)
        .build();
    let subscriber = WASMLayer::new(config);
    tracing_subscriber::registry().with(subscriber).init();
    // tracing::subscriber::set_global_default(subscriber).expect("set global");
}
