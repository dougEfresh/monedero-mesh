use crate::actors::{ClearPairing, RegisterDapp, RegisterTopicManager, RegisterWallet};
use crate::rpc::{Proposer, SessionProposeResponse};
use crate::{Dapp, Pairing, PairingManager, Result, SessionTopic, SubscriptionId, Topic, Wallet};
use tracing::{debug, info, warn};

impl PairingManager {
    pub(super) async fn restore_saved_pairing(&self) -> Result<()> {
        if let Some(pairing) = self.pairing() {
            info!("found existing topic {pairing}");
            let request_actor = self.actors.request();

            self.subscribe(pairing.topic.clone()).await?;
            info!("Checking if peer is alive");
            if !self.alive().await {
                info!("clearing pairing topics and sessions");
                self.relay.unsubscribe(pairing.topic.clone()).await?;
                if let Err(e) = request_actor.send(ClearPairing).await {
                    warn!("failed to clear pairing: '{e}'");
                }
                return Ok(());
            }
            request_actor
                .send(RegisterTopicManager(pairing.topic.clone(), self.clone()))
                .await?;
        }
        Ok(())
    }

    async fn register_pk(&self, pk: String) -> Result<SessionTopic> {
        let (session_topic, _) = self.ciphers.create_common_topic(pk)?;
        // TODO: Do I need the subscriptionId?
        self.subscribe(session_topic.clone()).await?;
        Ok(session_topic)
    }

    pub(crate) async fn register_wallet_pk(
        &self,
        controller: SessionProposeResponse,
    ) -> Result<Topic> {
        self.register_pk(controller.responder_public_key).await
    }

    pub(crate) async fn register_dapp_pk(&self, proposer: Proposer) -> Result<Topic> {
        self.register_pk(proposer.public_key).await
    }
}
