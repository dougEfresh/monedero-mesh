use std::str::FromStr;
use {
    base64::{prelude::BASE64_STANDARD, Engine},
    crate::{Result, SolanaSession, WalletConnectTransaction},
    solana_pubkey::Pubkey,
    solana_signature::{Signature},
    solana_signer::{Signer, SignerError},
    std::{
        fmt::{Debug, Display, Formatter},
        time::Duration,
    },
    tokio::sync::oneshot::error::TryRecvError,
    tracing::{debug, warn},
};

struct ChannelProps {
    tx: tokio::sync::oneshot::Sender<Result<Signature>>,
    message: Vec<u8>,
}

#[derive(Clone)]
pub struct ReownSigner {
    session: SolanaSession,
    tx: tokio::sync::mpsc::UnboundedSender<ChannelProps>,
}

impl Debug for ReownSigner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "signer[{}]", self.session.pubkey())
    }
}

impl Display for ReownSigner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "signer[{}]", self.session.pubkey())
    }
}

impl PartialEq for ReownSigner {
    fn eq(&self, other: &Self) -> bool {
        self.session.eq(&other.session)
    }
}

impl Signer for ReownSigner {
    fn try_pubkey(&self) -> std::result::Result<Pubkey, SignerError> {
        Ok(self.session.pubkey())
    }

    #[tracing::instrument(level = "info", skip(message))]
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
                        if cnt > 30 {
                            return Err(SignerError::Custom("signer timeout".to_string()));
                        }
                        std::thread::sleep(Duration::from_secs(5));
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
    signer: ReownSigner,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<ChannelProps>,
) {
    while let Some(props) = rx.recv().await {
        let result = signer.wc_sign_transaction(props.message).await;
        if let Err(_) = props.tx.send(result) {
            warn!("singer has been dropped!");
        }
    }
}

impl ReownSigner {
    pub fn new(session: SolanaSession) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<ChannelProps>();
        let wc_signer = ReownSigner { session, tx };
        tokio::spawn(handler_signer(wc_signer.clone(), rx));
        wc_signer
    }

    pub async fn wc_sign_transaction(&self, msg: Vec<u8>) -> Result<Signature> {
        let encoded = BASE64_STANDARD.encode(msg);
        let sol_tx_req = WalletConnectTransaction {
            transaction: encoded,
        };
        self.session.sign_transaction(sol_tx_req).await
    }
}
