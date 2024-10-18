use std::str::FromStr;

use promkit::crossterm::style::{Attribute, Attributes, Color, Stylize};
use promkit::style::StyleBuilder;
use strum::IntoEnumIterator;

use crate::cmd::{prompts, TokenMenu};
use crate::context::Context;

pub async fn invoke(context: &Context) -> anyhow::Result<()> {
    let tc = context.wallet.token_accounts_client();
    let accounts = tc.accounts().await?.accounts;
    let mut p = promkit::preset::query_selector::QuerySelector::new(
        &accounts,
        |text, items| -> Vec<String> {
            items
                .into_iter()
                .map(|i| i.to_lowercase())
                .filter(|q| text.starts_with(q.as_str()))
                .collect::<Vec<String>>()
        },
    )
    .prompt()?;
    let item = p.run()?;
    let token = accounts
        .iter()
        .find(|t| *t.metadata.symbol == item)
        .unwrap();
    let title = format!(
        "{}        {}   {}",
        token.metadata.symbol,
        token.address,
        token.account.token_amount.real_number_string()
    );
    let menu = TokenMenu::iter();
    let style = StyleBuilder::new()
        .fgc(Color::Blue)
        .bgc(Color::White)
        .attrs(Attributes::none().with(Attribute::Framed))
        .build()
        .bold();
    let mut p = promkit::preset::listbox::Listbox::new(menu)
        .title(title)
        .title_style(style)
        .prompt()?;
    let item: TokenMenu = TokenMenu::from_str(p.run()?.as_str()).expect("should not happen");
    let token_transfer_client = context.wallet.token_transfer_client(token);
    let result = match item {
        TokenMenu::Transfer => {
            let to = prompts::pubkey()?;
            let (amt, lamports) = prompts::amount(token.account.token_amount.decimals)?;
            if prompts::confirm_send(&to, amt, context)? {
                let sig = token_transfer_client.transfer(&to, lamports).await?;
                prompts::signature(sig, context)?;
            }
            Ok(())
        }
        _ => Ok(()),
    };
    result
}
