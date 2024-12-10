use {
    async_trait::async_trait,
    monedero_mesh::{SocketEvent, SocketListener},
    std::sync::{Arc, Mutex},
};
mod test_utils;
use test_utils::*;

#[allow(dead_code)]
struct DummySocketListener {
    pub events: Arc<Mutex<Vec<SocketEvent>>>,
}

#[async_trait]
impl SocketListener for DummySocketListener {
    async fn handle_socket_event(&self, event: SocketEvent) {
        let mut l = self.events.lock().unwrap();
        l.push(event);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_relay_pair_ping() -> anyhow::Result<()> {
    let test_components = init_test_components().await?;
    let dapp = test_components.dapp;
    dapp.pair_ping().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_relay_pair_delete() -> anyhow::Result<()> {
    let test_components = init_test_components().await?;
    let dapp = test_components.dapp;
    dapp.purge().await?;
    // TODO verify purge
    // let c = dapp.ciphers();
    // yield_ms(2000).await;
    // assert!(c.pairing().is_none());
    // let dapp_actors = test_components.dapp_actors;
    // let components = dapp_actors.request().send(RegisteredComponents).await?;
    // assert!(components);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_relay_pair_extend() -> anyhow::Result<()> {
    let test_components = init_test_components().await?;
    let dapp = test_components.dapp;
    dapp.extend(100_000).await?;
    Ok(())
}

//#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
// async fn test_relay_disconnect() -> anyhow::Result<()> {
//    let test_components = init_test_components().await?;
//    let dapp = test_components.dapp;
//    let listener = DummySocketListener::new();
//    dapp.register_socket_listener(listener.clone()).await;
//    // special topic to indicate a force disconnect
//    let disconnect_topic =
//        Topic::from("
// 92b2701dbdbb72abea51591a06d41e7d76ebfe18e1a1ca5680a5ac6e3717c6d9");
//    dapp.subscribe(disconnect_topic.clone()).await?;
//    yield_ms(1000).await;
//    assert_matches!(
//        dapp.pair_ping().await,
//        Err(monedero_mesh::Error::ConnectError(
//            monedero_mesh::ClientError::Disconnected
//        ))
//    );
//    info!("waiting for reconnect");
//    yield_ms(3300).await;
//    // should have reconnected
//    dapp.ping().await?;
//    let l = listener.events.lock().unwrap();
//    assert_eq!(2, l.len());
//    Ok(())
//}
