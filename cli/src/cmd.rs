pub mod transfer;

use enum_str_derive::EnumStr;

pub mod prompts {
    use crate::context::{Confirmation, Context};
    use solana_sdk::native_token::LAMPORTS_PER_SOL;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    pub fn amount() -> anyhow::Result<(f64, u64)> {
        let mut p = promkit::preset::readline::Readline::default()
            .validator(
                |text| text.parse::<f64>().is_ok(),
                |text| format!("invalid amount {}", text.parse::<f64>().err().unwrap()),
            )
            .prompt()?;
        let amt: f64 = p.run()?.parse()?;
        let lamports: u64 = (amt * LAMPORTS_PER_SOL as f64) as u64;
        Ok((amt, lamports))
    }

    pub fn pubkey() -> anyhow::Result<Pubkey> {
        let mut p = promkit::preset::readline::Readline::default()
            .enable_history()
            .validator(
                |text| Pubkey::from_str(text).is_ok(),
                |text| {
                    format!(
                        "invalid public key {}",
                        Pubkey::from_str(&text).err().unwrap()
                    )
                },
            )
            .prompt()?;
        Ok(Pubkey::from_str(p.run()?.as_str())?)
    }

    pub fn confirm(msg: &str, ctx: &Context) -> anyhow::Result<bool> {
        let mut p = promkit::preset::confirm::Confirm::new(msg).prompt()?;
        let confirm: Confirmation = p.run()?.into();
        Ok(confirm.proceed())
    }
}
#[derive(EnumStr)]
pub enum MainMenu {
    Transactions,
    Transfer,
    Tokens,
    Swap,
    Stake,
    Logs,
    Quit,
}
