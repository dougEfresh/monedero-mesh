use {
    tracing_error::ErrorLayer,
    tracing_subscriber::{
        fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, Layer,
    },
};

pub fn initialize_logging() {
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG").unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME"))),
    );

    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_line_number(false)
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());

    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .init();
}
