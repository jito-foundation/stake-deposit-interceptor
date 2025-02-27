use solana_client::nonblocking::rpc_client::RpcClient;
use solana_metrics::datapoint_info;
use solana_sdk::clock::DEFAULT_SLOTS_PER_EPOCH;
use stake_deposit_interceptor::state::DepositReceipt;
use std::sync::Arc;
use tracing::error;

use crate::CrankerError;

pub fn emit_error(message: String) {
    error!(message);
    datapoint_info!("sdi-test-error", ("message", message, String),);
}

pub async fn emit_heartbeat(rpc_client: Arc<RpcClient>, tick: u64) -> Result<(), CrankerError> {
    let current_slot = rpc_client.get_slot().await?;
    let current_epoch = rpc_client.get_epoch_info().await?.epoch;
    let epoch_percentage =
        (current_slot as f64 % DEFAULT_SLOTS_PER_EPOCH as f64) / DEFAULT_SLOTS_PER_EPOCH as f64;

    datapoint_info!(
        "sdi-test-tick",
        ("tick", tick, i64),
        ("current-epoch", current_epoch, i64),
        ("current-slot", current_slot, i64),
        ("epoch-percentage", epoch_percentage, f64),
    );

    Ok(())
}

pub fn emit_crank(
    deposit_receipts: u64,
    future_deposits: u64,
    not_yet_expired_receipts: u64,
    claimed_receipts: u64,
) {
    datapoint_info!(
        "sdi-test-crank",
        ("total-deposit-receipts", deposit_receipts, i64),
        ("future-deposits", future_deposits, i64),
        ("not-yet-expired-receipts", not_yet_expired_receipts, i64),
        ("claimed-receipts", claimed_receipts, i64),
    );
}

pub fn emit_deposit_receipt(deposit_receipt: &DepositReceipt) {
    let base = deposit_receipt.base.to_string();
    let cool_down_seconds: u64 = deposit_receipt.cool_down_seconds.into();
    let deposit_time: u64 = deposit_receipt.deposit_time.into();
    let initial_fee_bps: u32 = deposit_receipt.initial_fee_bps.into();
    let lst_amount: u64 = deposit_receipt.lst_amount.into();
    let owner = deposit_receipt.owner.to_string();
    let stake_pool = deposit_receipt.stake_pool.to_string();
    let stake_pool_deposit_stake_authority = deposit_receipt
        .stake_pool_deposit_stake_authority
        .to_string();

    let account_string = format!("{:?}", deposit_receipt);
    datapoint_info!(
        "sdi-test-deposit-receipt",
        ("base", base, String),
        ("cool-down-seconds", cool_down_seconds, i64),
        ("deposit-time", deposit_time, i64),
        ("initial-fee-bps", initial_fee_bps, i64),
        ("lst-amount", lst_amount, i64),
        ("owner", owner, String),
        ("stake-pool", stake_pool, String),
        (
            "stake-pool-deposit-stake-authority",
            stake_pool_deposit_stake_authority,
            String
        ),
        ("account-string", account_string, String),
    );
}
