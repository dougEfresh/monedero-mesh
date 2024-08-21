use tokio::sync::oneshot;
use crate::{relay, Cipher, KvStorage, Pairing, PairingManager};
use crate::pair::settlement;
use crate::relay::mock::MOCK_FACTORY;
use crate::rpc::{SessionProposeRequest, SessionProposeResponse, SessionRequestRequest, SessionSettleRequest};
use crate::transport::{RpcRecv, TopicTransport};
use crate::Result;

pub trait WalletSettlementHandler: Send + 'static {
  fn handle_propose(&mut self, settlement: RpcRecv<SessionProposeRequest>) -> Result<SessionSettleRequest>;
}

pub trait WalletHandler: Send + 'static {
  fn handle_request(&mut self, request: RpcRecv<SessionRequestRequest>) {}
}

#[derive(Clone)]
pub struct WalletManager {
  mgr: PairingManager,
  ciphers: Cipher,
  transporter: TopicTransport
}
pub struct WalletSession {

}

impl WalletManager {
  pub async fn init(mgr: PairingManager) -> Self {
    Self {
      transporter: mgr.transporter(),
      ciphers: mgr.ciphers(),
      mgr,
    }
  }

  pub async fn pair<S, H>(&self, pairing: Pairing, settler: S, handler: H) -> Result<WalletSession>
  where
    S: WalletSettlementHandler,
    H: WalletHandler
  {
    self.ciphers.set_pairing(Some(pairing.clone()))?;
    let key = match self.ciphers.public_key_hex() {
      None => return Err(crate::error::Error::PairingInitError),
      Some(k) => k,
    };

    let (tx, rx) = oneshot::channel::<Result<()>>();

    tokio::spawn(settlement::start_settlement(self.mgr.clone(), tx, self.broadcast_tx.clone()));
    let _ = self.relay.subscribe(pairing.topic.clone()).await?;

    Ok(rx)
  }
}

pub struct WalletAdapter {

}

pub(crate) struct  MockWallet {
  //client: relay::Client,
}

impl MockWallet {

  fn new() -> Self {
    Self {

    }
  }
}