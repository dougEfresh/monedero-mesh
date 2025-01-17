mod setup;
use {
    setup::{explorer, init_test_components},
    solana_pubkey::Pubkey,
    std::str::FromStr,
    tracing::info,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_instructions() -> anyhow::Result<()> {
    let wallet = init_test_components().await?;
    let to = Pubkey::from_str("E4SfgGV2v9GLYsEkCQhrrnFbBcYmAiUZZbJ7swKGzZHJ")?;
    let instructions = vec![
        spl_memo::build_memo(b"testing", &[wallet.pk()]),
        solana_sdk::system_instruction::transfer(wallet.pk(), &to, 1),
    ];
    let table = Pubkey::from_str("8E2BPQ4bQx4btVYv6WCXKXzBkNuBEe3oLZDGUifF3eKR")?;
    info!("using lookup table {table}");
    let sig = wallet
        .send_instructions(&instructions, None)
        //.send_instructions(&instructions, Some(&table))
        .await?;
    explorer(&sig);
    Ok(())
}
