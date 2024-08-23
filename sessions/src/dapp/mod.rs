use crate::rpc::ProposeNamespaces;
use crate::{Pairing, PairingManager};
use std::sync::Arc;

#[derive(Clone)]
pub struct Dapp {
    manager: PairingManager,
}

impl Dapp {
    pub fn new(manager: PairingManager) -> Self {
        Self { manager }
    }
    /*
    pub async fn propose(
      &self,
      namespaces: ProposeNamespaces,
    ) -> Result<(Arc<Pairing>, crate::EventClientSession)> {
      let pairing: Pairing = self.manager.create_pairing_topic()
      let cipher = self.ciphers.clone();
      let pairing: Pairing = Default::default();
      cipher.set_pairing(Some(pairing.clone()))?;
      let pairing = Arc::new(pairing);
      self.relay.subscribe(pairing.topic.clone()).await?;
      let key = match self.ciphers.public_key_hex() {
        None => return Err(crate::error::Error::PairingInitError),
        Some(k) => k,
      };
      let payload = RequestParams::SessionPropose(SessionProposeRequest {
        relays: vec![RelayProtocol::default()],
        proposer: Proposer::new(key, self.metadata.clone()),
        required_namespaces: namespaces,
      });

      let (tx, rx) = oneshot::channel::<Result<ClientSession>>();

      tokio::spawn(settlement::process_settlement(
          self.clone(),
          tx,
          self.broadcast_tx.clone(),
          pairing.clone(),
          payload,
      ));

      Ok((pairing, rx))
    }
     */
}
