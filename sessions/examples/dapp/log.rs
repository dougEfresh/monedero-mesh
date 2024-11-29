use {
    tracing_error::ErrorLayer,
    tracing_subscriber::{
        fmt::format::FmtSpan,
        layer::SubscriberExt,
        util::SubscriberInitExt,
        Layer,
    },
};

pub fn initialize_logging() -> anyhow::Result<()> {
    let app_name = env!("CARGO_PKG_NAME");
    let app = microxdg::XdgApp::new(app_name)?;
    let directory = app.app_data()?;
    std::fs::create_dir_all(directory.clone())?;
    let log_file_name = format!("{}.log", env!("CARGO_CRATE_NAME"));
    let log_path = directory.join(log_file_name.clone());
    let log_file = std::fs::File::create(log_path)?;
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG")
            //.or_else(|_| std::env::var(LOG_ENV.clone()))
            .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME"))),
    );

    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(false)
        .with_line_number(false)
        .with_writer(log_file)
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());

    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .init();
    Ok(())
}
