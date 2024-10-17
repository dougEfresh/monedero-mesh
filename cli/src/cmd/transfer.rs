use crate::cmd::prompts;
use crate::context::Context;
use solana_sdk::pubkey::Pubkey;

pub async fn invoke(context: &Context) -> anyhow::Result<()> {
    let to: Pubkey = prompts::pubkey()?;
    let (amt, lamports) = prompts::amount()?;
    let msg = format!("Send {} SOL to {}?", amt, to);
    let proceed = prompts::confirm(&msg, context)?;
    if !proceed {
        return Ok(());
    }
    let sig = context.wallet.transfer(&to, lamports).await?;
    let msg = format!("Signature: {sig} Open on solscan.io?");
    let proceed = prompts::confirm(&msg, context)?;
    if proceed {
        return Ok(());
    }
    Ok(())
}
