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

    pub(crate) async fn register_dapp_session_topic(
        &self,
        dapp: Dapp,
        topic: SessionTopic,
    ) -> Result<SubscriptionId> {
        self.actors
            .request()
            .send(RegisterDapp(topic.clone(), dapp))
            .await?;
        self.subscribe(topic).await
    }

    pub(crate) async fn register_wallet_pairing(
        &self,
        wallet: Wallet,
        pairing: Pairing,
    ) -> Result<()> {
        debug!("registering wallet to topic {}", pairing.topic);
        self.set_pairing(pairing.clone()).await?;
        self.actors
            .request()
            .send(RegisterWallet(pairing.topic.clone(), wallet))
            .await?;
        Ok(())
    }

    pub(crate) async fn register_wallet_pk(
        &self,
        dapp: Dapp,
        controller: SessionProposeResponse,
    ) -> Result<Topic> {
        let (session_topic, _) = self
            .ciphers
            .create_common_topic(controller.responder_public_key)?;
        self.actors
            .request()
            .send(RegisterDapp(session_topic.clone(), dapp))
            .await?;
        // TODO: Do I need the subscriptionId?
        self.subscribe(session_topic.clone()).await?;
        Ok(session_topic)
    }

    pub(crate) async fn register_dapp_pk(
        &self,
        wallet: Wallet,
        proposer: Proposer,
    ) -> Result<Topic> {
        let (session_topic, _) = self.ciphers.create_common_topic(proposer.public_key)?;
        self.actors
            .request()
            .send(RegisterWallet(session_topic.clone(), wallet))
            .await?;
        // TODO: Do I need the subscriptionId?
        let id = self.subscribe(session_topic.clone()).await?;
        info!("subscribed to topic {session_topic:#?} with id {id:#?}");
        Ok(session_topic)
    }
}
