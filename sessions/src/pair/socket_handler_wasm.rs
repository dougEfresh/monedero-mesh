use {
    crate::{spawn_task, wait::wait_until, PairingManager, SocketEvent},
    tokio::sync::mpsc,
    tracing::warn,
};

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
                let reconnector = mgr.clone();
                if let Err(e) = wait_until(1000, async move {
                    reconnector.open_socket().await
                    // if let Err(e) = reconnector.open_socket().await {
                    //    warn!("failed to reconnect ${e}");
                    //}
                })
                .await
                {
                    warn!("failed to reconnect ${e}");
                }
                // spawn_task(async move {
                //    if let Err(e) = reconnector.open_socket().await {
                //        warn!("failed to reconnect ${e}");
                //    }
                //});

                // warn!("TODO implement connection retry");
            }
        }
    }
}
