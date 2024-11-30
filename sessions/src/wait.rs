use crate::Error::WaitError;

#[cfg(not(target_family = "wasm"))]
pub async fn wait_until<F, T>(duration_ms: u32, future: F) -> crate::Result<T>
where
    F: std::future::Future<Output = T>,
{
    match tokio::time::timeout(std::time::Duration::from_millis(duration_ms as u64), future).await {
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
