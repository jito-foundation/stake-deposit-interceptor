use std::{
    num::NonZeroU32,
    time::{SystemTime, UNIX_EPOCH},
};

use jito_bytemuck::AccountDeserialize;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_client::{rpc_client::RpcClient, rpc_config::CommitmentConfig};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use solana_system_interface::instruction::transfer;
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_stake_pool::{find_stake_program_address, find_withdraw_authority_program_address};
use stake_deposit_interceptor_program::{
    instruction::{
        create_claim_pool_tokens_instruction, create_deposit_stake_instruction,
        create_init_deposit_stake_authority_instruction, derive_stake_pool_deposit_stake_authority,
    },
    state::{
        hopper::Hopper, DepositReceipt, StakeDepositInterceptorDiscriminators,
        StakePoolDepositStakeAuthority,
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
        format!("Invalid stake_deposit_authority {stake_deposit_authority_address}: {err}")
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
        &stake_deposit_interceptor_program::id(),
        &config.fee_payer.pubkey(),
        stake_pool_address,
        &stake_pool.pool_mint,
        &spl_stake_pool::id(),
        &spl_token_interface::id(),
        fee_wallet,
        cool_down_seconds,
        initial_fee_bps,
        authority,
        &base.pubkey(),
    );

    let (deposit_stake_authority_pubkey, _bump_seed) = derive_stake_pool_deposit_stake_authority(
        &stake_deposit_interceptor_program::id(),
        stake_pool_address,
        &base.pubkey(),
    );

    let base_signer: Box<dyn Signer> = Box::new(base);
    let transaction =
        checked_transaction_with_signers(config, &[ix], &[&config.fee_payer, &base_signer])?;
    send_transaction(config, transaction)?;
    println!("Created stake_deposit_authority:");
    print!("{deposit_stake_authority_pubkey:?}");
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
        solana_stake_interface::state::StakeStateV2::Stake(_, stake, _) => {
            Ok(stake.delegation.voter_pubkey)
        }
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
        &stake_deposit_interceptor_program::id(),
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
        &spl_token_interface::id(),
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
    pub _stake_pool: Pubkey,
    pub deposit_time: u64,
    pub _cool_down_seconds: u64,
    pub _expiry_time: u64,
    pub is_expired: bool,
    pub lst_amount: u64,
    pub current_fee_amount: u64,
    pub owner_ata_exists: bool,
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
        .get_program_ui_accounts_with_config(
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
        .map_err(|e| format!("RPC error: {e}"))?;

    let mut receipts = Vec::new();
    for (pubkey, account) in accounts {
        let account_data = account.data.decode().unwrap();
        match DepositReceipt::try_from_slice_unchecked(account_data.as_slice()) {
            Ok(receipt) => receipts.push((pubkey, *receipt)),
            Err(e) => eprintln!("Failed to deserialize receipt for {pubkey}: {e}"),
        }
    }

    Ok(receipts)
}

/// Calculate receipt status and timing information
pub fn calculate_receipt_info(
    rpc_client: &RpcClient,
    receipt_address: Pubkey,
    receipt: &DepositReceipt,
) -> ReceiptInfo {
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

    // Check if owner has an ATA for the J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn token
    let jitosol_mint = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn"
        .parse::<Pubkey>()
        .unwrap();
    let owner_ata = get_associated_token_address(&receipt.owner, &jitosol_mint);
    let owner_ata_exists = rpc_client.get_account(&owner_ata).is_ok();

    ReceiptInfo {
        receipt_address,
        base: receipt.base,
        owner: receipt.owner,
        _stake_pool: receipt.stake_pool,
        deposit_time,
        _cool_down_seconds: cool_down_seconds,
        _expiry_time: expiry_time,
        is_expired,
        lst_amount: u64::from(receipt.lst_amount),
        current_fee_amount,
        owner_ata_exists,
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
    let default_program_id = stake_deposit_interceptor_program::id();
    let program_id = program_id.unwrap_or(&default_program_id);

    let receipts = get_all_deposit_receipts(&config.rpc_client, program_id, stake_pool)?;

    if receipts.is_empty() {
        println!("No deposit receipts found.");
        return Ok(());
    }

    let mut receipt_infos: Vec<ReceiptInfo> = receipts
        .into_iter()
        .map(|(addr, receipt)| calculate_receipt_info(&config.rpc_client, addr, &receipt))
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
    println!("{:-<170}", "");
    println!(
        "{:<45} {:<45} {:<45} {:<10} {:<15} {:<10}",
        "Receipt Address", "Base", "Owner", "Status", "LST Amount", "JitoSOL ATA"
    );
    println!("{:-<170}", "");

    for info in &receipt_infos {
        let status = if info.is_expired { "EXPIRED" } else { "ACTIVE" };
        let ata_status = if info.owner_ata_exists {
            "EXISTS"
        } else {
            "MISSING"
        };
        println!(
            "{:<45} {:<45} {:<45} {:<10} {:<15} {:<10}",
            info.receipt_address, info.base, info.owner, status, info.lst_amount, ata_status
        );

        if !info.is_expired && info.current_fee_amount > 0 {
            println!(
                "  └─ Current fee if claimed now: {}",
                info.current_fee_amount
            );
        }
    }

    println!("\nSummary: {receipt_count} receipts found");
    Ok(())
}

/// Command to claim pool tokens for a specific deposit receipt
pub fn command_claim_tokens(
    config: &Config,
    receipt_address: &Pubkey,
    destination: Option<&Pubkey>,
    after_cooldown: bool,
    create_ata: bool,
) -> CommandResult {
    // Get the receipt data
    let receipt_account = config
        .rpc_client
        .get_account(receipt_address)
        .map_err(|e| format!("Failed to get receipt account: {e}"))?;

    let receipt = DepositReceipt::try_from_slice_unchecked(receipt_account.data.as_slice())
        .map_err(|e| format!("Failed to deserialize receipt: {e}"))?;

    // Determine after_cooldown automatically: true if fee payer is not the owner
    let auto_after_cooldown = config.fee_payer.pubkey() != receipt.owner;
    let final_after_cooldown = after_cooldown || auto_after_cooldown;

    if auto_after_cooldown && !after_cooldown {
        println!("Note: Setting after_cooldown=true because fee payer ({}) is not the receipt owner ({})",
                 config.fee_payer.pubkey(), receipt.owner);
    }

    // Get the stake pool deposit authority
    let authority_account = config
        .rpc_client
        .get_account(&receipt.stake_pool_deposit_stake_authority)
        .map_err(|e| format!("Failed to get deposit authority account: {e}"))?;

    let stake_pool_deposit_authority =
        StakePoolDepositStakeAuthority::try_from_slice_unchecked(authority_account.data.as_slice())
            .map_err(|e| format!("Failed to deserialize deposit authority: {e}"))?;

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

    // Collect all instructions
    let mut instructions = Vec::new();

    // Check if destination account exists, add creation instruction if needed
    if config
        .rpc_client
        .get_account(&destination_token_account)
        .is_err()
    {
        if create_ata {
            println!("Will create destination token account: {destination_token_account}");

            let create_ata_ix =
                spl_associated_token_account_interface::instruction::create_associated_token_account(
                    &config.fee_payer.pubkey(),
                    &receipt.owner,
                    &stake_pool_deposit_authority.pool_mint,
                    &spl_token_interface::id(),
                );
            instructions.push(create_ata_ix);
        } else {
            return Err(format!(
                "Destination token account {destination_token_account} does not exist. Use --create-ata to create it."
            )
            .into());
        }
    }

    // Check if fee account exists, add creation instruction if needed
    if config
        .rpc_client
        .get_account(&fee_wallet_token_account)
        .is_err()
    {
        println!("Will create fee wallet token account: {fee_wallet_token_account}");

        let create_fee_ata_ix =
            spl_associated_token_account_interface::instruction::create_associated_token_account(
                &config.fee_payer.pubkey(),
                &stake_pool_deposit_authority.fee_wallet,
                &stake_pool_deposit_authority.pool_mint,
                &spl_token_interface::id(),
            );
        instructions.push(create_fee_ata_ix);
    }

    // Create the claim instruction
    let claim_ix = create_claim_pool_tokens_instruction(
        &stake_deposit_interceptor_program::id(),
        receipt_address,
        &receipt.owner,
        &stake_pool_deposit_authority.vault,
        &destination_token_account,
        &fee_wallet_token_account,
        &receipt.stake_pool_deposit_stake_authority,
        &stake_pool_deposit_authority.pool_mint,
        &spl_token_interface::id(),
        final_after_cooldown,
    );
    instructions.push(claim_ix);

    let transaction =
        checked_transaction_with_signers(config, &instructions, &[config.fee_payer.as_ref()])?;

    match send_transaction(config, transaction) {
        Ok(_) => {
            println!("Successfully claimed pool tokens for receipt {receipt_address}");
            println!("Tokens sent to: {destination_token_account}");
            println!("After cooldown: {final_after_cooldown}");
            Ok(())
        }
        Err(e) => Err(format!("Failed to claim pool tokens: {e}").into()),
    }
}

/// Command to get [`StakePoolDepositStakeAuthority`]
pub fn command_get_stake_deposit_authority(
    config: &Config,
    stake_deposit_authority_address: &Pubkey,
) -> CommandResult {
    let stake_deposit_authority =
        get_stake_deposit_authority(&config.rpc_client, stake_deposit_authority_address)?;

    println!("\nStake Pool Deposit Stake Authority");
    println!("=====================================");
    println!("Base:                    {}", stake_deposit_authority.base);
    println!(
        "Stake Pool:              {}",
        stake_deposit_authority.stake_pool
    );
    println!(
        "Pool Mint:               {}",
        stake_deposit_authority.pool_mint
    );
    println!(
        "Authority:               {}",
        stake_deposit_authority.authority
    );
    println!("Vault:                   {}", stake_deposit_authority.vault);
    println!(
        "Stake Pool Program ID:   {}",
        stake_deposit_authority.stake_pool_program_id
    );
    let cool_down_seconds: u64 = stake_deposit_authority.cool_down_seconds.into();
    println!("Cool Down Seconds:       {cool_down_seconds}");
    let initial_fee_bps: u32 = stake_deposit_authority.inital_fee_bps.into();
    println!("Initial Fee (bps):       {initial_fee_bps}",);
    println!(
        "Fee Wallet:              {}",
        stake_deposit_authority.fee_wallet
    );
    println!(
        "Bump Seed:               {}",
        stake_deposit_authority.bump_seed
    );

    Ok(())
}

pub fn command_fund_hopper(
    config: &Config,
    interceptor_program_id: &Pubkey,
    whitelist_management_program_id: &Pubkey,
    base: &Pubkey,
    lamports: u64,
) -> CommandResult {
    let whitelist_pda = Pubkey::find_program_address(
        &[b"whitelist", base.to_bytes().as_slice()],
        whitelist_management_program_id,
    )
    .0;
    let hopper_pda = Hopper::find_program_address(interceptor_program_id, &whitelist_pda).0;

    let ix = transfer(&config.fee_payer.pubkey(), &hopper_pda, lamports);

    let transaction =
        checked_transaction_with_signers(config, &[ix], &[config.fee_payer.as_ref()])?;

    match send_transaction(config, transaction) {
        Ok(_) => {
            println!("Successfully transferring",);
            Ok(())
        }
        Err(e) => Err(format!("Failed to fund hopper: {e}").into()),
    }
}

pub fn command_hopper_balance(
    config: &Config,
    interceptor_program_id: &Pubkey,
    whitelist_management_program_id: &Pubkey,
    base: &Pubkey,
) -> CommandResult {
    let whitelist_pda = Pubkey::find_program_address(
        &[b"whitelist", base.to_bytes().as_slice()],
        whitelist_management_program_id,
    )
    .0;
    let hopper_pda = Hopper::find_program_address(interceptor_program_id, &whitelist_pda).0;
    let hopper_acc = config.rpc_client.get_account(&hopper_pda)?;

    println!("Hopper Balance: {}", hopper_acc.lamports);

    Ok(())
}
