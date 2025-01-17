mod setup;
use {
    setup::{explorer, init_test_components},
    solana_pubkey::Pubkey,
    std::str::FromStr,
    tracing::info,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_solana_session() -> anyhow::Result<()> {
    let wallet = init_test_components().await?;
    let to = Pubkey::from_str("E4SfgGV2v9GLYsEkCQhrrnFbBcYmAiUZZbJ7swKGzZHJ")?;
    let balance = wallet.balance().await?;
    info!("settlement complete {wallet} has balance {balance}");
    info!("sending SOL to {to}");
    let sig = wallet.transfer(&to, 1).await?;
    explorer(&sig);
    Ok(())
}
