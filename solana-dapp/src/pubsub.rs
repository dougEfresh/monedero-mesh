use std::sync::Arc;
use std::time::Duration;
use futures::StreamExt;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_client::rpc_response::RpcKeyedAccount;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use tokio::select;
use tracing::instrument;
use crate::account::AccountType;
use crate::state::AccountTx;

#[derive(Clone)]
pub struct PubSub {
    client: Arc<PubsubClient>,
    account_tx: Arc<AccountTx>,
}

impl PubSub {

    pub async fn new(url: &str, account_tx: AccountTx) -> anyhow::Result<Self> {
        let ps = PubsubClient::new(url).await?;
        Ok(Self {
            client: Arc::new(ps),
            account_tx: Arc::new(account_tx),
        })
    }

    pub async fn slots(&self, mut rx: tokio::sync::broadcast::Receiver<bool>) {
        if let Err(e) = self.slots_internal(rx).await {
            tracing::error!("failed to subscribe to slots {e}");
        }
    }

    async fn slots_internal(&self, mut rx: tokio::sync::broadcast::Receiver<bool>) -> anyhow::Result<()> {
        let (mut sub, unsub) = self.client.slot_subscribe().await?;
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = rx.recv()  => {
                    break;
                }
                _ = interval.tick() => {
                    if let Some(s) = sub.next().await {
                        tracing::trace!("slot {}", s.slot);
                    }
                }
            }
        }
        unsub().await;
        tracing::debug!("leaving slot update");
        Ok(())
    }


    pub async fn run(&self, acct: AccountType, rx: tokio::sync::broadcast::Receiver<bool>) {
        if let Err(e) = self.run_internal(acct, rx).await {
            tracing::error!("failed to subscribe to accounts {}", e);
        }
    }

    #[instrument(name = "pubsub", level = "info", skip(self, rx))]
    async fn run_internal(&self, acct: AccountType, mut rx: tokio::sync::broadcast::Receiver<bool>) -> anyhow::Result<()> {
        let (mut sub, unsub) = self.client.account_subscribe(acct.pubkey(), None).await?;
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        tracing::info!("ws subscribe");
        loop {
            select! {
                _ = rx.recv()  => {
                    break;
                }
                _ = interval.tick() => {
                    if let Some(s) = sub.next().await {
                        match acct {
                            AccountType::Native(_) => {
                                if let Err(err) = self.account_tx.balance_tx.send(s.value.lamports) {
                                    tracing::warn!("balance channel has closed!");
                                }
                            },
                            _ => {
                                tracing::info!("lamports {}", s.value.lamports);
                            }
                        }

                    }
                }
            }
        }
        unsub().await;
        tracing::info!("leaving account sub");
        Ok(())
    }
}
