use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
use tokio::sync::mpsc;

use crate::mock::mocker::Mocker;
use crate::mock::{ConnectionCategory, ConnectionPair};
use crate::{ConnectionHandler, Message, MessageIdGenerator};

pub static MOCK_FACTORY: Lazy<MockerFactory> = Lazy::new(MockerFactory::new);

pub type ConnectionPairChannels = (
    mpsc::UnboundedSender<Message>,
    mpsc::UnboundedReceiver<Message>,
);

pub struct MockerFactory {
    broadcasters: Arc<Mutex<HashMap<ConnectionPair, ConnectionPairChannels>>>,
    generator: MessageIdGenerator,
}

impl MockerFactory {
    pub fn new() -> Self {
        Self {
            broadcasters: Arc::new(Mutex::new(HashMap::new())),
            generator: MessageIdGenerator::new(),
        }
    }

    /// A pair of clients should get the same broadcast channel
    /// Otherwise create a new one
    pub fn create<T: ConnectionHandler>(
        &self,
        handler: T,
        connection_id: &ConnectionPair,
    ) -> Mocker {
        if let Ok(mut l) = self.broadcasters.lock() {
            if let Some((tx, rx)) = l.remove(connection_id) {
                tracing::info!("found a pair for {}", connection_id);
                return Mocker::new(
                    handler,
                    self.generator.clone(),
                    connection_id.clone(),
                    tx,
                    rx,
                );
            }
            // create new channels, store the otherside in the hashmap
            tracing::info!("creating new connection pair for mock {}", connection_id);
            let (dapp_tx, dapp_rx) = mpsc::unbounded_channel::<Message>();
            let (wallet_tx, wallet_rx) = mpsc::unbounded_channel::<Message>();
            return match &connection_id.1 {
                ConnectionCategory::Dapp => {
                    let wallet_id =
                        ConnectionPair(connection_id.0.clone(), ConnectionCategory::Wallet);
                    l.insert(wallet_id, (dapp_tx, wallet_rx));
                    Mocker::new(
                        handler,
                        self.generator.clone(),
                        connection_id.clone(),
                        wallet_tx,
                        dapp_rx,
                    )
                }
                ConnectionCategory::Wallet => {
                    let dapp_id = ConnectionPair(connection_id.0.clone(), ConnectionCategory::Dapp);
                    l.insert(dapp_id, (wallet_tx, dapp_rx));
                    Mocker::new(
                        handler,
                        self.generator.clone(),
                        connection_id.clone(),
                        dapp_tx,
                        wallet_rx,
                    )
                }
            };
        }
        panic!("failed to get lock for mock factory!!");
    }
}
