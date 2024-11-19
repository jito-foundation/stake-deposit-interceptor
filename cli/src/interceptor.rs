use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use stake_deposit_interceptor::instruction::create_init_deposit_stake_authority_instruction;

use crate::{
    checked_transaction_with_signers, get_stake_pool, send_transaction, CommandResult, Config,
};

/// Create a StakePoolStakeDepositAuthority on the
/// stake-pool-interceptor program.
pub fn command_create_stake_deposit_authority(
    config: &Config,
    stake_pool_address: &Pubkey,
    fee_wallet: &Pubkey,
    cool_down_seconds: u64,
    initial_fee_bps: u32,
    authority: &Pubkey,
) -> CommandResult {
    // Ephemeral keypair used for stake_deposit_authority PDA seed.
    let base = Keypair::new();
    let stake_pool = get_stake_pool(&config.rpc_client, stake_pool_address)?;
    let ix = create_init_deposit_stake_authority_instruction(
        &stake_deposit_interceptor::id(),
        &config.fee_payer.pubkey(),
        stake_pool_address,
        &stake_pool.pool_mint,
        &spl_stake_pool::id(),
        &spl_token::id(),
        fee_wallet,
        cool_down_seconds,
        initial_fee_bps,
        authority,
        &base.pubkey(),
    );

    let transaction = checked_transaction_with_signers(config, &[ix], &[&base])?;
    send_transaction(config, transaction)?;
    Ok(())
}
