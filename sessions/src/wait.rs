use crate::Error::WaitError;

#[allow(clippy::option_if_let_else)]
#[cfg(not(target_family = "wasm"))]
pub async fn wait_until<F, T>(duration_ms: u32, future: F) -> crate::Result<T>
where
    F: std::future::Future<Output = T> + Send,
{
    match tokio::time::timeout(
        std::time::Duration::from_millis(u64::from(duration_ms)),
        future,
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(_) => Err(WaitError(duration_ms)),
    }
}

#[cfg(target_family = "wasm")]
pub async fn wait_until<F, T>(duration_ms: u32, future: F) -> crate::Result<T>
where
    F: std::future::Future<Output = T>,
{
    use {
        futures_util::future::{select, Either},
        gloo_timers::future::TimeoutFuture,
    };
    let pinned_future = Box::pin(future);
    let timeout_future = TimeoutFuture::new(duration_ms);

    match select(pinned_future, timeout_future).await {
        Either::Left((v, _)) => Ok(v),
        Either::Right(_) => Err(WaitError(duration_ms)),
    }
}
