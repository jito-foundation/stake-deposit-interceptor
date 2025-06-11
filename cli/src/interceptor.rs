use std::num::NonZeroU32;
use std::time::{SystemTime, UNIX_EPOCH};

use jito_bytemuck::AccountDeserialize;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer, stake,
};
use spl_associated_token_account::get_associated_token_address;
use spl_stake_pool::{find_stake_program_address, find_withdraw_authority_program_address};
use stake_deposit_interceptor::{
    instruction::{
        create_claim_pool_tokens_instruction, create_deposit_stake_instruction,
        create_init_deposit_stake_authority_instruction, derive_stake_pool_deposit_stake_authority,
    },
    state::{
        DepositReceipt, StakeDepositInterceptorDiscriminators, StakePoolDepositStakeAuthority,
    },
};

use crate::{
    checked_transaction_with_signers, get_stake_pool, get_stake_state, get_validator_list,
    send_transaction, CommandResult, Config, Error,
};

macro_rules! unique_signers {
    ($vec:ident) => {
        $vec.sort_by_key(|l| l.pubkey());
        $vec.dedup();
    };
}

fn get_stake_deposit_authority(
    rpc_client: &RpcClient,
    stake_deposit_authority_address: &Pubkey,
) -> Result<StakePoolDepositStakeAuthority, Error> {
    let account_data = rpc_client.get_account_data(stake_deposit_authority_address)?;
    let stake_deposit_authority = StakePoolDepositStakeAuthority::try_from_slice_unchecked(
        account_data.as_slice(),
    )
    .map_err(|err| {
        format!(
            "Invalid stake_deposit_authority {}: {}",
            stake_deposit_authority_address, err
        )
    })?;
    Ok(*stake_deposit_authority)
}

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

    let (deposit_stake_authority_pubkey, _bump_seed) = derive_stake_pool_deposit_stake_authority(
        &stake_deposit_interceptor::id(),
        stake_pool_address,
        &base.pubkey(),
    );

    let base_signer: Box<dyn Signer> = Box::new(base);
    let transaction =
        checked_transaction_with_signers(config, &[ix], &[&config.fee_payer, &base_signer])?;
    send_transaction(config, transaction)?;
    println!("Created stake_deposit_authority:");
    print!("{:?}", deposit_stake_authority_pubkey);
    Ok(())
}

/// Deposit a stake account through the interceptor program
pub fn command_deposit_stake(
    config: &Config,
    stake_deposit_authority_address: &Pubkey,
    stake: &Pubkey,
    withdraw_authority: Box<dyn Signer>,
    referrer_token_account: &Option<Pubkey>,
) -> CommandResult {
    let stake_deposit_authority =
        get_stake_deposit_authority(&config.rpc_client, stake_deposit_authority_address)?;

    // Most below is copy/pasta from `command_deposit_stake` with very slight modifications.
    let stake_pool = get_stake_pool(&config.rpc_client, &stake_deposit_authority.stake_pool)?;
    let stake_state = get_stake_state(&config.rpc_client, stake)?;

    let vote_account = match stake_state {
        stake::state::StakeStateV2::Stake(_, stake, _) => Ok(stake.delegation.voter_pubkey),
        _ => Err("Wrong stake account state, must be delegated to validator"),
    }?;
    // Check if this vote account has staking account in the pool
    let validator_list = get_validator_list(&config.rpc_client, &stake_pool.validator_list)?;
    let validator_stake_info = validator_list
        .find(&vote_account)
        .ok_or("Vote account not found in the stake pool")?;
    let validator_seed = NonZeroU32::new(validator_stake_info.validator_seed_suffix.into());

    // Calculate validator stake account address linked to the pool
    let (validator_stake_account, _) = find_stake_program_address(
        &spl_stake_pool::id(),
        &vote_account,
        &stake_deposit_authority.stake_pool,
        validator_seed,
    );

    println!(
        "Depositing stake {} into stake pool {} via stake_deposit_authority {}",
        stake, stake_deposit_authority.stake_pool, stake_deposit_authority_address
    );

    let mut signers = vec![config.fee_payer.as_ref(), withdraw_authority.as_ref()];

    let referrer_token_account = referrer_token_account.unwrap_or(stake_deposit_authority.vault);

    let pool_withdraw_authority = find_withdraw_authority_program_address(
        &spl_stake_pool::id(),
        &stake_deposit_authority.stake_pool,
    )
    .0;

    // Finally create interceptor instructions

    // Ephemoral keypair for PDA seed of DepositReceipt
    let deposit_receipt_base = Keypair::new();

    println!("Created DepositReceipt {}", deposit_receipt_base.pubkey());

    let ixs = create_deposit_stake_instruction(
        &stake_deposit_interceptor::id(),
        &config.fee_payer.pubkey(),
        &spl_stake_pool::id(),
        &stake_deposit_authority.stake_pool,
        &stake_pool.validator_list,
        &pool_withdraw_authority,
        stake,
        &withdraw_authority.pubkey(),
        &validator_stake_account,
        &stake_pool.reserve_stake,
        &stake_deposit_authority.vault,
        &stake_pool.manager_fee_account,
        &referrer_token_account,
        &stake_pool.pool_mint,
        &spl_token::id(),
        &deposit_receipt_base.pubkey(),
        &stake_deposit_authority.base,
    );
    signers.push(&deposit_receipt_base);

    unique_signers!(signers);

    let transaction = checked_transaction_with_signers(config, &ixs, &signers)?;
    send_transaction(config, transaction)?;
    Ok(())
}

// Data structure to hold receipt information for display
#[derive(Debug)]
pub struct ReceiptInfo {
    pub receipt_address: Pubkey,
    pub base: Pubkey,
    pub owner: Pubkey,
    pub stake_pool: Pubkey,
    pub deposit_time: u64,
    pub _cool_down_seconds: u64,
    pub _expiry_time: u64,
    pub is_expired: bool,
    pub lst_amount: u64,
    pub current_fee_amount: u64,
}

/// Get all deposit receipts for the program, optionally filtered by stake pool
pub fn get_all_deposit_receipts(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    stake_pool_filter: Option<&Pubkey>,
) -> Result<Vec<(Pubkey, DepositReceipt)>, Error> {
    let discriminator = StakeDepositInterceptorDiscriminators::DepositReceipt as u8;

    let mut filters = vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        0,
        &[discriminator],
    ))];

    // Add stake pool filter if provided
    if let Some(stake_pool) = stake_pool_filter {
        // DepositReceipt has stake_pool at offset 64 (after base=32 + owner=32)
        filters.push(RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
            64,
            stake_pool.as_ref(),
        )));
    }

    let accounts = rpc_client
        .get_program_accounts_with_config(
            program_id,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    commitment: Some(CommitmentConfig::confirmed()),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(|e| format!("RPC error: {}", e))?;

    let mut receipts = Vec::new();
    for (pubkey, account) in accounts {
        match DepositReceipt::try_from_slice_unchecked(account.data.as_slice()) {
            Ok(receipt) => receipts.push((pubkey, *receipt)),
            Err(e) => eprintln!("Failed to deserialize receipt for {}: {}", pubkey, e),
        }
    }

    Ok(receipts)
}

/// Calculate receipt status and timing information
pub fn calculate_receipt_info(receipt_address: Pubkey, receipt: &DepositReceipt) -> ReceiptInfo {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let deposit_time = u64::from(receipt.deposit_time);
    let cool_down_seconds = u64::from(receipt.cool_down_seconds);
    let expiry_time = deposit_time.saturating_add(cool_down_seconds);
    let is_expired = now > expiry_time;

    let current_fee_amount = if is_expired {
        0
    } else {
        receipt.calculate_fee_amount(now as i64)
    };

    ReceiptInfo {
        receipt_address,
        base: receipt.base,
        owner: receipt.owner,
        stake_pool: receipt.stake_pool,
        deposit_time,
        _cool_down_seconds: cool_down_seconds,
        _expiry_time: expiry_time,
        is_expired,
        lst_amount: u64::from(receipt.lst_amount),
        current_fee_amount,
    }
}

/// Command to list all deposit receipts with their status
pub fn command_list_receipts(
    config: &Config,
    program_id: Option<&Pubkey>,
    stake_pool: Option<&Pubkey>,
    show_expired_only: bool,
    show_active_only: bool,
) -> CommandResult {
    let default_program_id = stake_deposit_interceptor::id();
    let program_id = program_id.unwrap_or(&default_program_id);

    let receipts = get_all_deposit_receipts(&config.rpc_client, program_id, stake_pool)?;

    if receipts.is_empty() {
        println!("No deposit receipts found.");
        return Ok(());
    }

    let mut receipt_infos: Vec<ReceiptInfo> = receipts
        .into_iter()
        .map(|(addr, receipt)| calculate_receipt_info(addr, &receipt))
        .collect();

    // Apply filters
    if show_expired_only {
        receipt_infos.retain(|info| info.is_expired);
    } else if show_active_only {
        receipt_infos.retain(|info| !info.is_expired);
    }

    if receipt_infos.is_empty() {
        println!("No receipts match the specified filters.");
        return Ok(());
    }

    // Sort by deposit time (newest first)
    receipt_infos.sort_by(|a, b| b.deposit_time.cmp(&a.deposit_time));

    let receipt_count = receipt_infos.len();

    // Display results
    println!("\nDeposit Receipts:");
    println!("{:-<150}", "");
    println!(
        "{:<45} {:<45} {:<45} {:<10} {:<15}",
        "Receipt Address", "Base", "Owner", "Status", "LST Amount"
    );
    println!("{:-<150}", "");

    for info in &receipt_infos {
        let status = if info.is_expired { "EXPIRED" } else { "ACTIVE" };
        println!(
            "{:<45} {:<45} {:<45} {:<10} {:<15}",
            info.receipt_address, info.base, info.owner, status, info.lst_amount
        );

        if !info.is_expired && info.current_fee_amount > 0 {
            println!(
                "  └─ Current fee if claimed now: {}",
                info.current_fee_amount
            );
        }
    }

    println!("\nSummary: {} receipts found", receipt_count);
    Ok(())
}

/// Command to claim pool tokens for a specific deposit receipt
pub fn command_claim_tokens(
    config: &Config,
    receipt_address: &Pubkey,
    destination: Option<&Pubkey>,
    after_cooldown: bool,
) -> CommandResult {
    // Get the receipt data
    let receipt_account = config
        .rpc_client
        .get_account(receipt_address)
        .map_err(|e| format!("Failed to get receipt account: {}", e))?;

    let receipt = DepositReceipt::try_from_slice_unchecked(receipt_account.data.as_slice())
        .map_err(|e| format!("Failed to deserialize receipt: {}", e))?;

    // Get the stake pool deposit authority
    let authority_account = config
        .rpc_client
        .get_account(&receipt.stake_pool_deposit_stake_authority)
        .map_err(|e| format!("Failed to get deposit authority account: {}", e))?;

    let stake_pool_deposit_authority =
        StakePoolDepositStakeAuthority::try_from_slice_unchecked(authority_account.data.as_slice())
            .map_err(|e| format!("Failed to deserialize deposit authority: {}", e))?;

    // Determine the destination token account
    let destination_token_account = match destination {
        Some(dest) => *dest,
        None => {
            get_associated_token_address(&receipt.owner, &stake_pool_deposit_authority.pool_mint)
        }
    };

    // Get fee wallet token account
    let fee_wallet_token_account = get_associated_token_address(
        &stake_pool_deposit_authority.fee_wallet,
        &stake_pool_deposit_authority.pool_mint,
    );

    // Check if fee account exists, create if not
    if config
        .rpc_client
        .get_account(&fee_wallet_token_account)
        .is_err()
    {
        println!(
            "Creating fee wallet token account: {}",
            fee_wallet_token_account
        );

        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &config.fee_payer.pubkey(),
                &stake_pool_deposit_authority.fee_wallet,
                &stake_pool_deposit_authority.pool_mint,
                &spl_token::id(),
            );

        let recent_blockhash = config.rpc_client.get_latest_blockhash()?;
        let create_ata_tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&config.fee_payer.pubkey()),
            &[config.fee_payer.as_ref()],
            recent_blockhash,
        );

        config
            .rpc_client
            .send_and_confirm_transaction(&create_ata_tx)?;
        println!("Created fee wallet token account");
    }

    // Create the claim instruction
    let claim_ix = create_claim_pool_tokens_instruction(
        &stake_deposit_interceptor::id(),
        receipt_address,
        &receipt.owner,
        &stake_pool_deposit_authority.vault,
        &destination_token_account,
        &fee_wallet_token_account,
        &receipt.stake_pool_deposit_stake_authority,
        &stake_pool_deposit_authority.pool_mint,
        &spl_token::id(),
        after_cooldown,
    );

    let transaction =
        checked_transaction_with_signers(config, &[claim_ix], &[config.fee_payer.as_ref()])?;

    match send_transaction(config, transaction) {
        Ok(_) => {
            println!(
                "Successfully claimed pool tokens for receipt {}",
                receipt_address
            );
            println!("Tokens sent to: {}", destination_token_account);
            Ok(())
        }
        Err(e) => Err(format!("Failed to claim pool tokens: {}", e).into()),
    }
}
