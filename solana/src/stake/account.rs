use solana_program::instruction::Instruction;
use solana_program::message::Message;
use solana_program::pubkey::Pubkey;
use solana_program::stake::state::{Authorized, StakeStateV2};
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::stake::instruction::{self as stake_instruction};
use solana_sdk::stake::{self};
use solana_sdk::transaction::Transaction;

use crate::Error::{AccountExists, BadParameter, MinimumDelegation};
use crate::{Result, StakeClient, WithMemo};

impl StakeClient {
    pub(crate) async fn create_instructions(
        &self,
        seed: impl AsRef<str>,
        lamports: u64,
    ) -> Result<(Pubkey, Vec<Instruction>)> {
        let min_amt = self.minimum_delegation().await?;
        if lamports < min_amt {
            return Err(MinimumDelegation {
                amt: lamports,
                min_amt,
            });
        }
        let stake_account = self.session.pubkey();
        let stake_account_address =
            Pubkey::create_with_seed(&stake_account, seed.as_ref(), &stake::program::id())?;
        if self.rpc.get_account(&stake_account_address).await.is_ok() {
            return Err(AccountExists(stake_account_address.to_string()));
        }

        let minimum_balance = self
            .rpc
            .get_minimum_balance_for_rent_exemption(StakeStateV2::size_of())
            .await?;
        if lamports < minimum_balance {
            return Err(BadParameter(format!(
                "need at least {minimum_balance} lamports for stake account to be rent exempt, provided lamports: {lamports}"
            )));
        }

        let authorized = Authorized {
            staker: self.session.pubkey(),
            withdrawer: self.session.pubkey(),
        };
        let inxs = stake_instruction::create_account_with_seed_checked(
            &stake_account,
            &stake_account_address,
            &stake_account,
            seed.as_ref(),
            &authorized,
            lamports,
        )
        .with_memo(Some(&self.memo));
        Ok((stake_account_address, inxs))
    }

    pub async fn create(
        &self,
        seed: impl AsRef<str>,
        lamports: u64,
    ) -> Result<(Pubkey, Signature)> {
        let (stake_account, instructions) = self.create_instructions(seed, lamports).await?;
        let msg = Message::new(&instructions, Some(&self.signer.pubkey()));
        let hash = self.rpc.get_latest_blockhash().await?;
        let mut tx = Transaction::new_unsigned(msg);
        tx.try_sign(&[&self.signer], hash)?;
        let sig = self.rpc.send_and_confirm_transaction(&tx).await?;
        Ok((stake_account, sig))
    }
}
