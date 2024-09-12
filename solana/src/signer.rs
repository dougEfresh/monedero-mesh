use crate::{serialize_raw_message, Result, SolanaSession, WalletConnectTransaction};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Signature, SignerError};
use solana_sdk::signer::Signer;
use solana_sdk::signers::Signers;
use solana_sdk::transaction::Transaction;
use std::time::Duration;
use tokio::sync::oneshot::error::TryRecvError;
use tracing::{debug, warn};
use monedero_namespaces::ChainId;
use monedero_mesh::ClientSession;

struct ChannelProps {
    tx: tokio::sync::oneshot::Sender<Result<Signature>>,
    message: Vec<u8>,
}

#[derive(Clone)]
pub struct WalletConnectSigner {
    session: SolanaSession,
    tx: tokio::sync::mpsc::UnboundedSender<ChannelProps>,
}

impl Signer for WalletConnectSigner {
    fn try_pubkey(&self) -> std::result::Result<Pubkey, SignerError> {
        Ok(self.session.pubkey())
    }

    fn try_sign_message(&self, message: &[u8]) -> std::result::Result<Signature, SignerError> {
        let (tx, mut rx) = tokio::sync::oneshot::channel::<Result<Signature>>();
        let channel_prop = ChannelProps {
            tx,
            message: message.to_vec(),
        };
        if let Err(e) = self.tx.send(channel_prop) {
            return Err(SignerError::Custom(format!("{e}")));
        }
        let mut cnt = 0;
        loop {
            cnt += 1;
            match rx.try_recv() {
                Ok(r) => match r {
                    Ok(sig) => return Ok(sig),
                    Err(e) => return Err(SignerError::Custom(format!("{e}"))),
                },
                Err(e) => match e {
                    TryRecvError::Empty => {
                        debug!("channel is empty retry: ({cnt})");
                        if cnt > 4 {
                            return Err(SignerError::Custom("signer timeout".to_string()));
                        }
                        std::thread::sleep(std::time::Duration::from_secs(5));
                    }
                    TryRecvError::Closed => {
                        return Err(SignerError::Custom("signer timeout".to_string()))
                    }
                },
            };
        }
    }

    fn is_interactive(&self) -> bool {
        true
    }
}

async fn handler_signer(
    signer: WalletConnectSigner,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<ChannelProps>,
) {
    while let Some(props) = rx.recv().await {
        let result = signer.wc_sign_transaction(props.message).await;
        if let Err(_) = props.tx.send(result) {
            warn!("singer has been dropped!");
        }
    }
}

impl WalletConnectSigner {
    pub fn new(session: SolanaSession) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<ChannelProps>();
        let wc_signer = WalletConnectSigner { session, tx };
        tokio::spawn(handler_signer(wc_signer.clone(), rx));
        wc_signer
    }

    pub async fn wc_sign_transaction(&self, msg: Vec<u8>) -> Result<Signature> {
        let encoded = serialize_raw_message(msg)?;
        let sol_tx_req = WalletConnectTransaction {
            transaction: encoded,
        };
        let sig = self.session.send_wallet_connect(sol_tx_req).await?;
        Signature::try_from(sig)
    }
}
