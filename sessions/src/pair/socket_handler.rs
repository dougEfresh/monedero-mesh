use {
    crate::{PairingManager, SocketEvent},
    tokio::sync::mpsc,
    tracing::{debug, info, warn},
};

use backoff::{future::retry, ExponentialBackoffBuilder};
use std::time::Duration;

async fn retry_backoff(mgr: PairingManager) {
    info!("reconnecting");
    tokio::time::sleep(Duration::from_secs(3)).await;
    let backoff = ExponentialBackoffBuilder::new()
        .with_max_elapsed_time(Some(Duration::from_secs(60)))
        .with_initial_interval(Duration::from_secs(3))
        .build();
    match retry(backoff, || async {
        info!("attempting reconnect");
        Ok(mgr.open_socket().await?)
    })
    .await
    {
        Ok(()) => {
            debug!("re-subsribing");
            if let Err(e) = mgr.resubscribe().await {
                warn!("failed to resubscribe! {e}");
            }
        }
        Err(e) => {
            warn!("failed to reconnect: {e}");
        }
    }
}

pub async fn handle_socket(mgr: PairingManager, mut rx: mpsc::UnboundedReceiver<SocketEvent>) {
    while let Some(message) = rx.recv().await {
        match message {
            SocketEvent::Connected | SocketEvent::Disconnect => {
                let l = mgr.socket_listeners.lock().await;
                for listener in l.iter() {
                    listener.handle_socket_event(message.clone()).await;
                }
            }
            SocketEvent::ForceDisconnect => {
                let l = mgr.socket_listeners.lock().await;
                for listener in l.iter() {
                    listener.handle_socket_event(message.clone()).await;
                }
                drop(l);
                let mgr_backoff = mgr.clone();
                retry_backoff(mgr_backoff).await;
            }
        }
    }
}
