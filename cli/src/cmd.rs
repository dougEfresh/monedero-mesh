mod cli;
pub mod stake;
pub mod tokens;
pub mod transfer;

use enum_str_derive::EnumStr;

pub mod prompts {
    use std::str::FromStr;

    use solana_sdk::native_token::LAMPORTS_PER_SOL;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Signature;

    use crate::context::Context;

    pub fn signature(sig: Signature, context: &Context) -> anyhow::Result<()> {
        let msg = format!("Signature: {sig} Open on solscan.io?");
        confirm(&msg, context)?;
        Ok(())
    }

    pub fn amount(dec: u8) -> anyhow::Result<(f64, u64)> {
        let mut p = promkit::preset::readline::Readline::default()
            .prefix("Amount? ")
            .validator(
                |text| text.parse::<f64>().is_ok(),
                |text| format!("invalid amount {}", text.parse::<f64>().err().unwrap()),
            )
            .prompt()?;
        let amt: f64 = p.run()?.parse()?;
        let amt_dec: u64 = (amt * 10_f64.powi(dec as i32)) as u64;
        Ok((amt, amt_dec))
    }

    pub fn pubkey() -> anyhow::Result<Pubkey> {
        let mut p = promkit::preset::readline::Readline::default()
            .prefix("To? ")
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

    pub fn confirm_send(to: &Pubkey, amt: f64, ctx: &Context) -> anyhow::Result<bool> {
        let msg = format!("Send {} SOL to {}?", amt, to);
        confirm(&msg, ctx)
    }

    pub struct Confirmation(String);

    impl Confirmation {
        pub fn proceed(&self) -> bool {
            self.0.contains("y")
        }
    }

    impl From<String> for Confirmation {
        fn from(value: String) -> Self {
            Self(value)
        }
    }
}

#[derive(EnumStr, strum_macros::EnumIter)]
pub enum MainMenu {
    Transactions,
    Transfer,
    Tokens,
    Swap,
    Stake,
    Logs,
    Quit,
}

#[derive(EnumStr, strum_macros::EnumIter)]
pub enum TokenMenu {
    Transactions,
    Transfer,
    Mint,
    Create,
    Back,
}
