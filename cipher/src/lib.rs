pub mod cipher;
mod error;
pub mod payload;
mod session;

use std::sync::Once;

pub use {error::CipherError, session::SessionKey};

#[allow(dead_code)]
static INIT: Once = Once::new();

pub use cipher::Cipher;

#[cfg(test)]
pub(crate) mod test {
    use {
        super::INIT,
        tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
    };

    pub(crate) fn init_tracing() {
        INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_target(true)
                .with_level(true)
                .with_span_events(FmtSpan::CLOSE)
                .with_env_filter(EnvFilter::from_default_env())
                .init();
        });
    }
}
