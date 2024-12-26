use {
    crate::{
        cmd::{Cli, StakeCommand, SubCommands},
        config::AppConfig,
        context::Context,
        log::initialize_logging,
    },
    clap::Parser,
    console::Term,
    copypasta::{ClipboardContext, ClipboardProvider},
    monedero_mesh::{
        domain::{Pairing, ProjectId},
        Dapp,
        KvStorage,
        Metadata,
        NoopSessionHandler,
        ProposeFuture,
        ReownBuilder,
    },
    monedero_solana::{SolanaSession, SolanaWallet},
    solana_sdk::{
        message::Message,
        native_token::{lamports_to_sol, sol_to_lamports},
        pubkey::Pubkey,
        signature::Signature,
        stake::instruction::{self as stake_instruction},
        transaction::Transaction,
    },
    std::{io::Write, str::FromStr},
};

mod cmd;
mod config;
mod context;
mod log;

async fn init_dapp(cfg: AppConfig) -> anyhow::Result<(Pairing, ProposeFuture, bool)> {
    let project = ProjectId::from("5c9d8a326d3afb25ed1dff90f6d1807a");
    let storage_path = format!("{}", cfg.storage()?.display());
    let storage = KvStorage::file(Some(storage_path))?;
    let mgr = ReownBuilder::new(project).store(storage).build().await?;
    let dapp = Dapp::new(mgr, Metadata {
        name: env!("CARGO_BIN_NAME").to_string(),
        description: "monedero mesh cli dapp".to_string(),
        url: "https://github.com/dougeEfresh/monedero-mesh".to_string(),
        icons: vec![],
        verify_url: None,
        redirect: None,
    })
    .await?;

    let (p, fut, cached) = dapp.propose(NoopSessionHandler, &cfg.chains()).await?;
    Ok((p, fut, cached))
}

// async fn main_menu(mut ctx: Context) -> anyhow::Result<()> {
//    let menu_items = vec![
//        MainMenu::Transfer,
//        MainMenu::Tokens,
//        MainMenu::Stake,
//        MainMenu::Swap,
//        MainMenu::Quit,
//    ];
//    loop {
//        let mut p = promkit::preset::listbox::Listbox::new(&menu_items)
//            .title("Main Menu")
//            .prompt()?;
//        let item = MainMenu::from_str(&p.run()?).expect("blah");
//        let result = match item {
//            MainMenu::Transfer => cmd::transfer::invoke(&mut ctx).await,
//            MainMenu::Tokens => cmd::tokens::invoke(&mut ctx).await,
//            MainMenu::Stake => cmd::stake::invoke(&mut ctx).await,
//            MainMenu::Quit => break,
//            _ => Ok(()),
//        };
//        if let Err(e) = result {
//            tracing::error!("error! {}", e);
//            writeln!(ctx.term, "error: {}", e)?;
//            let mut p =
// promkit::preset::confirm::Confirm::new("Continue?").prompt()?;            let
// confirm = p.run()?;            if confirm.contains("n") {
//                break;
//            }
//        }
//    }
//    Ok(())
//}
//
pub async fn deactivate(wallet: &SolanaWallet, account: &Pubkey) -> anyhow::Result<Signature> {
    let pk = wallet.pk();
    let rpc = wallet.rpc();
    let instruction = stake_instruction::deactivate_stake(account, pk);
    let msg = Message::new(&[instruction], Some(pk));
    let hash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_unsigned(msg);
    tx.try_sign(&[wallet], hash)?;
    Ok(rpc.send_transaction(&tx.into()).await?)
}

pub async fn stake_withdraw(ctx: &mut Context, account: &Pubkey) -> anyhow::Result<Signature> {
    let pk = ctx.wallet.pk();
    let rpc = ctx.wallet.rpc();
    let bal = rpc.get_balance(account).await?;
    ctx.term
        .write_fmt(format_args!("you have {bal} lamports\n"))?;
    let instruction = stake_instruction::withdraw(account, pk, pk, bal, None);
    let msg = Message::new(&[instruction], Some(pk));
    let hash = rpc.get_latest_blockhash().await?;
    let mut tx = Transaction::new_unsigned(msg);
    ctx.term.write_fmt(format_args!(
        "num of signers {}\n",
        tx.message().signer_keys().len()
    ))?;
    tx.try_sign(&[&ctx.wallet], hash)?;
    Ok(rpc.send_transaction(&tx.into()).await?)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = Cli::parse();
    initialize_logging()?;
    let mut ctx = ClipboardContext::new().expect("Failed to open clipboard");
    let mut term = Term::stdout();

    let cfg = AppConfig::new(matches.config.clone(), matches.profile.clone())?;
    term.write_fmt(format_args!("{cfg}\n"))?;
    let (pairing, fut, cached) = init_dapp(cfg.clone()).await?;
    if !cached {
        ctx.set_contents(pairing.to_string())
            .expect("Failed to set clipboard");

        term.write_fmt(format_args!("Pairing: {pairing}\n"))?;
    }

    tracing::info!("restored from cache {:?}", cached);
    let cs = fut.await?;
    if let Err(e) = cs.ping().await {
        tracing::info!("ping error {e}");
    }

    let sol_session = SolanaSession::try_from(&cs)?;
    let rpc_client = cfg.solana_rpc_client();
    let wallet = SolanaWallet::new(sol_session.clone(), rpc_client.clone())?;

    let mut ctx = Context {
        wallet: wallet.clone(),
        term,
    };
    match matches.subcommands {
        None => {
            // let helius =
            // Pubkey::from_str("3X3dgst3b3eNPmJqH5mPxp4kYxhfLwJJNJSfWpVRGn7J")?;
            // let sig = deactivate(&wallet, &helius).await?;
            // ctx.term.write_line(&format!("sig {sig}"))?;
            // tokio::spawn(async move { cs.pinger(Duration::from_secs(15)).await });
            // main_menu(ctx).await
            Ok(())
        }
        Some(SubCommands::Transfer(cmd)) => match cmd.command {
            cmd::TransferCommand::Native(args) => {
                let to = Pubkey::from_str(&args.to)?;
                let lamports = sol_to_lamports(args.sol);
                wallet.transfer(&to, lamports).await?;
                Ok(())
            }
        },
        Some(SubCommands::Stake(cmd)) => {
            match cmd.command {
                StakeCommand::Withdraw(args) => {
                    let account = Pubkey::from_str(args.account.as_str())?;
                    let sig = stake_withdraw(&mut ctx, &account).await?;
                    ctx.term.write_line(&format!("sig {sig}"))?;
                }
            }
            Ok(())
        }
        Some(SubCommands::Fees) => {
            // let fees = ctx.wallet.fees().await?;
            // ctx.term.write_line(&format!("{:?}", fees))?;
            Ok(())
        }
        Some(SubCommands::Balance) => {
            let balance = lamports_to_sol(ctx.wallet.balance().await?);
            ctx.term.write_line(&format!("{balance}"))?;
            Ok(())
        }
        _ => Ok(()),
    }
}
