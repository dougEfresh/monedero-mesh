use {
    crate::{
        compute_budget::WithComputeUnit,
        Error::RpcRequestError,
        KeyedStakeState,
        Result,
        StakeClient,
        WithMemo,
    },
    solana_program::{instruction::Instruction, message::Message, pubkey::Pubkey},
    solana_rpc_client_api::{
        config::RpcGetVoteAccountsConfig,
        request::DELINQUENT_VALIDATOR_SLOT_DISTANCE,
        response::RpcVoteAccountStatus,
    },
    solana_sdk::{
        signature::Signature,
        signer::Signer,
        stake::instruction::{self as stake_instruction},
        transaction::Transaction,
    },
};

impl StakeClient {
    pub async fn minimum_delegation(&self) -> Result<u64> {
        Ok(self.rpc.get_stake_minimum_delegation().await?)
    }

    /// Create stake account and stake
    pub async fn create_delegate(
        &self,
        lamports: u64,
        vote: &Pubkey,
    ) -> Result<(Pubkey, Signature)> {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let (stake_account, mut inxs) = self.create_instructions(seed, lamports).await?;
        let mut create_inxs = self.delegate_acct(&stake_account, vote).await?;
        inxs.append(&mut create_inxs);
        let _budget = self.fee_service.simulate(&inxs).await?;
        // TODO fix compute budget
        // let inxs= inxs.with_compute_unit(budget);
        let msg = Message::new(&inxs, Some(&self.session.pk));
        let hash = self.rpc.get_latest_blockhash().await?;
        let mut tx = Transaction::new_unsigned(msg);
        tx.try_sign(&[&self.signer], hash)?;
        Ok((
            stake_account,
            self.rpc.send_and_confirm_transaction(&tx).await?,
        ))
    }

    async fn delegate_acct(
        &self,
        stake_account: &Pubkey,
        vote: &Pubkey,
    ) -> Result<Vec<Instruction>> {
        let get_vote_accounts_config = RpcGetVoteAccountsConfig {
            vote_pubkey: Some(vote.to_string()),
            keep_unstaked_delinquents: Some(true),
            commitment: Some(self.rpc.commitment()),
            ..RpcGetVoteAccountsConfig::default()
        };
        let RpcVoteAccountStatus {
            current,
            delinquent,
        } = self
            .rpc
            .get_vote_accounts_with_config(get_vote_accounts_config)
            .await?;
        // filter should return at most one result
        let rpc_vote_account = current
            .first()
            .or_else(|| delinquent.first())
            .ok_or(RpcRequestError(format!("Vote account not found: {vote}")))?;

        let activated_stake = rpc_vote_account.activated_stake;
        let root_slot = rpc_vote_account.root_slot;
        let min_root_slot = self.rpc.get_slot().await?;
        let min_root_slot = min_root_slot.saturating_sub(DELINQUENT_VALIDATOR_SLOT_DISTANCE);
        let sanity_check_result = if root_slot >= min_root_slot || activated_stake == 0 {
            Ok(())
        } else if root_slot == 0 {
            return Err(RpcRequestError(
                "Unable to delegate. Vote account has no root slot".to_string(),
            ));
        } else {
            Err(RpcRequestError(format!(
                "Unable to delegate.  Vote account appears delinquent because its current root \
                 slot, {root_slot}, is less than {min_root_slot}"
            )))
        };
        if let Err(e) = sanity_check_result {
            return Err(e);
        }
        Ok(vec![stake_instruction::delegate_stake(
            stake_account,
            &self.signer.pubkey(),
            vote,
        )])
    }

    pub async fn delegate(&self, account: &KeyedStakeState, vote: &Pubkey) -> Result<Signature> {
        let ixs = self
            .delegate_acct(&account.stake_pubkey, vote)
            .await?
            .with_memo(Some(&self.memo));
        let msg = Message::new(&ixs, Some(&self.session.pk));
        let hash = self.rpc.get_latest_blockhash().await?;
        let mut tx = Transaction::new_unsigned(msg);
        tx.try_sign(&[&self.signer], hash)?;
        Ok(self.rpc.send_and_confirm_transaction(&tx).await?)
    }
}
