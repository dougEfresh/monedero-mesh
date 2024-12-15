use {
    crate::{PairingManager, SocketEvent},
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
                warn!("TODO implement connection retry");
            }
        }
    }
}
