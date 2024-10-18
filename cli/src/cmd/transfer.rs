use solana_sdk::pubkey::Pubkey;

use crate::cmd::prompts;
use crate::context::Context;

pub async fn invoke(context: &Context) -> anyhow::Result<()> {
    let to: Pubkey = prompts::pubkey()?;
    let (amt, lamports) = prompts::amount(9)?;
    let msg = format!("Send {} SOL to {}?", amt, to);
    let proceed = prompts::confirm(&msg, context)?;
    if !proceed {
        return Ok(());
    }
    let sig = context.wallet.transfer(&to, lamports).await?;
    prompts::signature(sig, context)
}
