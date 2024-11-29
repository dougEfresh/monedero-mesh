use {
    crate::{cmd::prompts, context::Context},
    solana_sdk::pubkey::Pubkey,
    std::str::FromStr,
};

pub async fn invoke(context: &Context) -> anyhow::Result<()> {
    let sc = context.wallet.stake_client();
    let accounts = sc.accounts().await?;
    let mut p = promkit::preset::listbox::Listbox::new(&accounts)
        .title("stake accounts")
        .prompt()?;
    let result = p.run()?;
    let result: Vec<&str> = result.split(" ").collect();
    if result.is_empty() {
        return Err(anyhow::format_err!("No accounts found"));
    }
    let chosen = Pubkey::from_str(result[0])?;

    prompts::confirm(&format!("unstake {chosen}"), context)?;
    let stake_acct = accounts
        .iter()
        .find(|a| a.stake_pubkey == chosen)
        .expect("no stake account");
    // let sig = sc.withdraw(stake_acct).await?;
    let v = Pubkey::from_str("he1iusunGwqrNtafDtLdhsUQDFvo13z9sUa36PauBtk")?;
    let sig = sc.delegate(stake_acct, &v).await?;
    prompts::signature(sig, context)?;
    Ok(())
}
