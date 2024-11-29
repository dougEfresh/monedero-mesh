use {
    crate::Result,
    solana_program::{instruction::Instruction, message::Message, pubkey::Pubkey},
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_rpc_client_api::config::{
        RpcSimulateTransactionAccountsConfig,
        RpcSimulateTransactionConfig,
    },
    solana_sdk::{compute_budget::ComputeBudgetInstruction, transaction::Transaction},
    std::{
        fmt::{Debug, Formatter},
        sync::Arc,
    },
};

#[derive(Clone)]
pub struct FeeService {
    pk: Pubkey,
    rpc: Arc<RpcClient>,
    max_fee: u64,
}

impl Debug for FeeService {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FeeService[{}] maxFee={}", self.pk, self.max_fee)
    }
}

/// See [@solana-developers/helpers](https://www.npmjs.com/package/@solana-developers/helpers)
impl FeeService {
    pub fn new(pk: Pubkey, rpc: Arc<RpcClient>, max_fee: u64) -> Self {
        Self { pk, rpc, max_fee }
    }

    #[tracing::instrument(level = "info")]
    pub async fn compute_fee(&self) -> Result<u64> {
        let result: Vec<u64> = self
            .rpc
            .get_recent_prioritization_fees(&[])
            .await?
            .into_iter()
            .map(|f| f.prioritization_fee)
            .filter(|f| *f > 0)
            .collect();
        if result.is_empty() {
            return Ok(self.max_fee / 2);
        }
        let (sum, min, max) = result
            .iter()
            .fold((0u64, u64::MIN, u64::MAX), |(sum, min, max), &x| {
                (sum + x, max.max(x), min.min(x))
            });
        let avg = sum / result.len() as u64;
        tracing::info!(
            "recent prioritization fees avg:{} max:{} min:{}",
            avg,
            max,
            min
        );
        Ok(avg.min(self.max_fee))
    }

    /// See [@solana-developers/helpers](https://www.npmjs.com/package/@solana-developers/helpers)
    #[tracing::instrument(level = "info", skip(ix), fields(ix_len = ix.len()))]
    pub async fn simulate(&self, ix: &[Instruction]) -> Result<Option<u32>> {
        let mut test_instructions = vec![
            // Set an arbitrarily high number in simulation
            // so we can be sure the transaction will succeed
            // and get the real compute units used
            ComputeBudgetInstruction::set_compute_unit_limit(1400000),
        ];
        test_instructions.extend_from_slice(ix);
        let message = Message::new(&ix, Some(&self.pk));
        let tx = Transaction::new_unsigned(message);
        let accounts = Some(RpcSimulateTransactionAccountsConfig {
            encoding: None,
            addresses: vec![format!("{}", self.pk)],
        });
        let result = self
            .rpc
            .simulate_transaction_with_config(&tx, RpcSimulateTransactionConfig {
                sig_verify: false,
                replace_recent_blockhash: true,
                commitment: None,
                encoding: None,
                accounts,
                min_context_slot: None,
                inner_instructions: true,
            })
            .await?;
        let units: Option<u32> = result.value.units_consumed.map(|u| u as u32);
        tracing::debug!("simulate tx = {:?}", units);
        Ok(units)
    }
}
