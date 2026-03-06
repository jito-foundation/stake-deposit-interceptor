use std::{
    num::NonZeroU32,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;
use bincode::deserialize;
use jito_bytemuck::AccountDeserialize;
use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_program::{borsh1::try_from_slice_unchecked, sysvar::SysvarId};
use solana_pubkey::{pubkey, Pubkey};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, UiAccountEncoding},
    filter::{Memcmp, RpcFilterType},
};
use solana_signer::Signer;
use solana_system_interface::instruction::transfer;
use solana_transaction::{Instruction, Signers, Transaction};
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_stake_pool::{
    find_stake_program_address, find_withdraw_authority_program_address,
    state::{StakePool, ValidatorList},
};
use stake_deposit_interceptor_client::instructions::{
    ClaimPoolTokensBuilder, DepositStakeBuilder, DepositStakeWhitelistedBuilder,
    InitStakePoolDepositStakeAuthorityBuilder, UpdateStakePoolDepositStakeAuthorityBuilder,
    WithdrawStakeWhitelistedBuilder,
};
use stake_deposit_interceptor_program::state::{
    hopper::Hopper, StakeDepositInterceptorDiscriminators,
};

use crate::{
    cli_config::CliConfig,
    interceptor::{StakeDepositInterceptorActions, StakeDepositInterceptorCommands},
};

pub async fn get_stake_pool(
    rpc_client: &RpcClient,
    stake_pool_address: &Pubkey,
) -> anyhow::Result<StakePool> {
    let account_data = rpc_client.get_account_data(stake_pool_address).await?;
    let stake_pool = try_from_slice_unchecked::<StakePool>(account_data.as_slice())?;
    Ok(stake_pool)
}

pub async fn get_validator_list(
    rpc_client: &RpcClient,
    validator_list_address: &Pubkey,
) -> anyhow::Result<ValidatorList> {
    let account_data = rpc_client.get_account_data(validator_list_address).await?;
    let validator_list = try_from_slice_unchecked::<ValidatorList>(account_data.as_slice())?;
    Ok(validator_list)
}

pub(crate) async fn get_stake_state(
    rpc_client: &RpcClient,
    stake_address: &Pubkey,
) -> anyhow::Result<solana_stake_interface::state::StakeStateV2> {
    let account_data = rpc_client.get_account_data(stake_address).await?;
    let stake_state = deserialize(account_data.as_slice())?;
    Ok(stake_state)
}

/// Get all deposit receipts for the program, optionally filtered by stake pool
pub async fn get_all_deposit_receipts(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    stake_pool_filter: Option<Pubkey>,
) -> anyhow::Result<
    Vec<(
        Pubkey,
        stake_deposit_interceptor_program::state::DepositReceipt,
    )>,
> {
    let discriminator = StakeDepositInterceptorDiscriminators::DepositReceipt as u8;

    let mut filters = vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        0,
        &[discriminator],
    ))];

    // Add stake pool filter if provided
    if let Some(stake_pool) = stake_pool_filter {
        // DepositReceipt has stake_pool at offset 72 (after discriminator=8 + base=32 + owner=32)
        filters.push(RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
            72,
            stake_pool.to_bytes().as_ref(),
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
        .await?;

    let mut receipts = Vec::new();
    for (pubkey, account) in accounts {
        let account_data = account.data.decode().unwrap();
        match stake_deposit_interceptor_program::state::DepositReceipt::try_from_slice_unchecked(
            account_data.as_slice(),
        ) {
            Ok(receipt) => receipts.push((pubkey, *receipt)),
            Err(e) => eprintln!("Failed to deserialize receipt for {pubkey}: {e}"),
        }
    }

    Ok(receipts)
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

/// Calculate receipt status and timing information
pub async fn calculate_receipt_info(
    rpc_client: &RpcClient,
    receipt_address: Pubkey,
    receipt: &stake_deposit_interceptor_program::state::DepositReceipt,
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
    let owner_ata_exists = rpc_client.get_account(&owner_ata).await.is_ok();

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

#[allow(dead_code)]
pub struct StakeDepositInterceptorCliHandler {
    /// The configuration of CLI
    cli_config: CliConfig,

    /// The Pubkey of the stake deposit interceptor Program
    stake_deposit_interceptor_program_id: Pubkey,

    /// This will print out the raw TX instead of running it
    print_tx: bool,

    /// This will print out the account information in JSON format
    print_json: bool,

    /// This will print out the account information in JSON format with reserved space
    print_json_with_reserves: bool,
}

impl StakeDepositInterceptorCliHandler {
    pub const fn new(
        cli_config: CliConfig,
        stake_deposit_interceptor_program_id: Pubkey,
        print_tx: bool,
        print_json: bool,
        print_json_with_reserves: bool,
    ) -> Self {
        Self {
            cli_config,
            stake_deposit_interceptor_program_id,
            print_tx,
            print_json,
            print_json_with_reserves,
        }
    }

    pub async fn handle(&self, action: StakeDepositInterceptorCommands) -> anyhow::Result<()> {
        match action {
            StakeDepositInterceptorCommands::Interceptor {
                action:
                    StakeDepositInterceptorActions::CreateStakeDepositAuthority {
                        pool,
                        fee_wallet,
                        cool_down_seconds,
                        initial_fee_bps,
                        authority,
                        spl_stake_pool_program_id,
                    },
            } => {
                self.create_stake_deposit_authority(
                    &pool,
                    &fee_wallet,
                    cool_down_seconds,
                    initial_fee_bps,
                    &authority,
                    spl_stake_pool_program_id,
                )
                .await
            }
            StakeDepositInterceptorCommands::Interceptor {
                action:
                    StakeDepositInterceptorActions::UpdateStakeDepositAuthority {
                        stake_deposit_authority,
                        jito_whitelist_management_program_id,
                    },
            } => {
                self.update_stake_deposit_authority(
                    &stake_deposit_authority,
                    &jito_whitelist_management_program_id,
                )
                .await
            }
            StakeDepositInterceptorCommands::Interceptor {
                action:
                    StakeDepositInterceptorActions::DepositStake {
                        stake_deposit_authority,
                        stake_account,
                        withdraw_authority,
                        referrer,
                        spl_stake_pool_program_id,
                    },
            } => {
                self.deposit_stake(
                    &stake_deposit_authority,
                    &stake_account,
                    &withdraw_authority,
                    &referrer,
                    spl_stake_pool_program_id,
                )
                .await
            }
            StakeDepositInterceptorCommands::Interceptor {
                action:
                    StakeDepositInterceptorActions::ListReceipts {
                        program_id,
                        stake_pool,
                        show_expired_only,
                        show_active_only,
                    },
            } => {
                self.list_receipts(program_id, stake_pool, show_expired_only, show_active_only)
                    .await
            }
            StakeDepositInterceptorCommands::Interceptor {
                action:
                    StakeDepositInterceptorActions::ClaimTokens {
                        receipt_address,
                        destination,
                        after_cooldown,
                        create_ata,
                    },
            } => {
                self.claim_tokens(receipt_address, destination, after_cooldown, create_ata)
                    .await
            }
            StakeDepositInterceptorCommands::Interceptor {
                action:
                    StakeDepositInterceptorActions::GetStakeDepositAuthority {
                        stake_deposit_authority,
                    },
            } => {
                self.get_stake_deposit_authority(&stake_deposit_authority)
                    .await
            }
            StakeDepositInterceptorCommands::Interceptor {
                action:
                    StakeDepositInterceptorActions::DepositStakeWhitelisted {
                        whitelist,
                        stake_deposit_authority,
                        deposit_stake,
                        validator_stake,
                        spl_stake_pool_program_id,
                    },
            } => {
                self.deposit_stake_whitelisted(
                    whitelist,
                    stake_deposit_authority,
                    deposit_stake,
                    validator_stake,
                    spl_stake_pool_program_id,
                )
                .await
            }
            StakeDepositInterceptorCommands::Interceptor {
                action:
                    StakeDepositInterceptorActions::WithdrawStakeWhitelisted {
                        whitelist,
                        stake_deposit_authority,
                        stake_split_from,
                        stake_split_to,
                        user_stake_authority,
                        fee_rebate_recipient,
                        spl_stake_pool_program_id,
                        amount,
                    },
            } => {
                self.withdraw_stake_whitelisted(
                    whitelist,
                    stake_deposit_authority,
                    stake_split_from,
                    &stake_split_to,
                    user_stake_authority,
                    fee_rebate_recipient,
                    spl_stake_pool_program_id,
                    amount,
                )
                .await
            }
            StakeDepositInterceptorCommands::Interceptor {
                action:
                    StakeDepositInterceptorActions::FundHopper {
                        whitelist,
                        lamports,
                    },
            } => self.fund_hopper(whitelist, lamports).await,
            StakeDepositInterceptorCommands::Interceptor {
                action: StakeDepositInterceptorActions::HopperBalance { whitelist },
            } => self.hopper_balance(whitelist).await,
        }
    }

    fn deposit_stake_authority_address(&self, stake_pool: &Pubkey, base: Pubkey) -> Pubkey {
        let program_id = self.stake_deposit_interceptor_program_id;
        Pubkey::find_program_address(
            &[
                b"deposit_stake_authority",
                &stake_pool.to_bytes(),
                &base.to_bytes(),
            ],
            &program_id,
        )
        .0
    }

    /// Derive the DepositReceipt pubkey for a given program
    pub fn stake_deposit_receipt_address(
        &self,
        stake_pool: &Pubkey,
        base: &Pubkey,
    ) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"deposit_receipt", &stake_pool.to_bytes(), &base.to_bytes()],
            &self.stake_deposit_interceptor_program_id,
        )
    }

    /// Create a StakePoolStakeDepositAuthority on the
    /// stake-pool-interceptor program.
    pub async fn create_stake_deposit_authority(
        &self,
        stake_pool_address: &Pubkey,
        fee_wallet: &Pubkey,
        cool_down_seconds: u64,
        initial_fee_bps: u32,
        authority: &Pubkey,
        spl_stake_pool_program_id: Pubkey,
    ) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();
        // Ephemeral keypair used for stake_deposit_authority PDA seed.
        let base = Keypair::new();

        let stake_pool = get_stake_pool(&rpc_client, stake_pool_address).await?;

        let deposit_stake_authority_pda =
            self.deposit_stake_authority_address(stake_pool_address, base.pubkey());

        let mut ix_builder = InitStakePoolDepositStakeAuthorityBuilder::new();
        ix_builder
            .payer(self.cli_config.signer.pubkey())
            .deposit_stake_authority(deposit_stake_authority_pda)
            .vault_ata(get_associated_token_address(
                &deposit_stake_authority_pda,
                &stake_pool.pool_mint,
            ))
            .authority(*authority)
            .base(base.pubkey())
            .stake_pool(*stake_pool_address)
            .stake_pool_mint(stake_pool.pool_mint)
            .stake_pool_program(spl_stake_pool_program_id)
            .token_program(spl_token_interface::id())
            .associated_token_program(spl_associated_token_account_interface::program::id())
            .fee_wallet(*fee_wallet)
            .cool_down_seconds(cool_down_seconds)
            .initial_fee_bps(initial_fee_bps);
        let mut ix = ix_builder.instruction();
        ix.program_id = self.stake_deposit_interceptor_program_id;

        log::info!("Initializing Stake Deposit Authority parameters: {ix_builder:?}",);

        self.process_transaction(
            &[ix],
            &self.cli_config.signer.pubkey(),
            &[self.cli_config.signer.clone(), Arc::new(base)],
        )
        .await?;

        Ok(())
    }

    /// Update a StakePoolStakeDepositAuthority on the
    /// stake-pool-interceptor program.
    pub async fn update_stake_deposit_authority(
        &self,
        stake_deposit_authority_address: &Pubkey,
        jito_whitelist_management_program_id: &Pubkey,
    ) -> anyhow::Result<()> {
        let mut ix_builder = UpdateStakePoolDepositStakeAuthorityBuilder::new();
        ix_builder
            .deposit_stake_authority(*stake_deposit_authority_address)
            .authority(self.cli_config.signer.pubkey())
            .jito_whitelist_management_program_id(*jito_whitelist_management_program_id);
        let mut ix = ix_builder.instruction();
        ix.program_id = self.stake_deposit_interceptor_program_id;

        log::info!("Updating Stake Deposit Authority parameters: {ix_builder:?}",);

        self.process_transaction(
            &[ix],
            &self.cli_config.signer.pubkey(),
            std::slice::from_ref(&self.cli_config.signer),
        )
        .await?;

        Ok(())
    }

    /// Deposit a stake account through the interceptor program
    pub async fn deposit_stake(
        &self,
        stake_deposit_authority_address: &Pubkey,
        stake: &Pubkey,
        withdraw_authority: &Pubkey,
        referrer_token_account: &Option<Pubkey>,
        spl_stake_pool_program_id: Pubkey,
    ) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();
        let stake_deposit_authority_acc = rpc_client
            .get_account(stake_deposit_authority_address)
            .await?;
        let stake_deposit_authority = stake_deposit_interceptor_program::state::StakePoolDepositStakeAuthority::try_from_slice_unchecked(stake_deposit_authority_acc.data.as_slice())?;

        // Most below is copy/pasta from `command_deposit_stake` with very slight modifications.
        let stake_pool = get_stake_pool(&rpc_client, &stake_deposit_authority.stake_pool).await?;
        let stake_state = get_stake_state(&rpc_client, stake).await?;

        let vote_account = match stake_state {
            solana_stake_interface::state::StakeStateV2::Stake(_, stake, _) => {
                Ok(stake.delegation.voter_pubkey)
            }
            _ => Err(anyhow!(
                "Wrong stake account state, must be delegated to validator"
            )),
        }?;

        // Check if this vote account has staking account in the pool
        let validator_list = get_validator_list(&rpc_client, &stake_pool.validator_list).await?;

        let validator_stake_info = validator_list
            .find(&vote_account)
            .ok_or(anyhow!("Vote account not found in the stake pool"))?;
        let validator_seed = NonZeroU32::new(validator_stake_info.validator_seed_suffix.into());

        // Calculate validator stake account address linked to the pool
        let (validator_stake_account, _) = find_stake_program_address(
            &spl_stake_pool_program_id,
            &vote_account,
            &stake_deposit_authority.stake_pool,
            validator_seed,
        );

        println!(
            "Depositing stake {} into stake pool {} via stake_deposit_authority {}",
            stake, stake_deposit_authority.stake_pool, stake_deposit_authority_address
        );

        let referrer_token_account =
            referrer_token_account.unwrap_or(stake_deposit_authority.vault);

        let pool_withdraw_authority = find_withdraw_authority_program_address(
            &spl_stake_pool_program_id,
            &stake_deposit_authority.stake_pool,
        )
        .0;

        // Finally create interceptor instructions

        // Ephemoral keypair for PDA seed of DepositReceipt
        let deposit_receipt_base = Keypair::new();

        let deposit_receipt = self
            .stake_deposit_receipt_address(
                &stake_deposit_authority.stake_pool,
                &deposit_receipt_base.pubkey(),
            )
            .0;

        println!(
            "Created DepositReceipt PDA {} (base {})",
            deposit_receipt,
            deposit_receipt_base.pubkey()
        );

        let mut ix_builder = DepositStakeBuilder::new();
        ix_builder
            .payer(self.cli_config.signer.pubkey())
            .stake_pool_program(spl_stake_pool_program_id)
            .deposit_receipt(deposit_receipt)
            .stake_pool(stake_deposit_authority.stake_pool)
            .validator_stake_list(stake_pool.validator_list)
            .deposit_stake_authority(*stake_deposit_authority_address)
            .base(deposit_receipt_base.pubkey())
            .stake_pool_withdraw_authority(pool_withdraw_authority)
            .stake(*stake)
            .validator_stake_account(validator_stake_account)
            .reserve_stake_account(stake_pool.reserve_stake)
            .vault(stake_deposit_authority.vault)
            .manager_fee_account(stake_pool.manager_fee_account)
            .referrer_pool_tokens_account(referrer_token_account)
            .pool_mint(stake_pool.pool_mint)
            .clock(solana_clock::Clock::id())
            .stake_history(solana_stake_interface::stake_history::StakeHistory::id())
            .stake_program(solana_stake_interface::program::id())
            .owner(*withdraw_authority);
        let mut ix = ix_builder.instruction();
        ix.program_id = self.stake_deposit_interceptor_program_id;

        log::info!("Depositing Stake parameters: {ix_builder:?}",);

        self.process_transaction(
            &[ix],
            &self.cli_config.signer.pubkey(),
            &[
                self.cli_config.signer.clone(),
                Arc::new(deposit_receipt_base),
            ],
        )
        .await?;

        Ok(())
    }

    pub async fn get_stake_deposit_authority(
        &self,
        stake_deposit_authority_address: &Pubkey,
    ) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();
        let stake_deposit_authority_acc = rpc_client
            .get_account(stake_deposit_authority_address)
            .await?;
        let stake_deposit_authority = stake_deposit_interceptor_program::state::StakePoolDepositStakeAuthority::try_from_slice_unchecked(stake_deposit_authority_acc.data.as_slice())?;

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
            "Jito Whitelist Management Program ID:              {}",
            stake_deposit_authority.jito_whitelist_management_program_id
        );
        println!(
            "Bump Seed:               {}",
            stake_deposit_authority.bump_seed
        );

        Ok(())
    }

    /// Command to list all deposit receipts with their status
    pub async fn list_receipts(
        &self,
        program_id: Option<Pubkey>,
        stake_pool: Option<Pubkey>,
        show_expired_only: bool,
        show_active_only: bool,
    ) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();

        let default_program_id = stake_deposit_interceptor_program::id();
        let program_id = program_id.unwrap_or(default_program_id);

        let receipts = get_all_deposit_receipts(&rpc_client, &program_id, stake_pool).await?;

        if receipts.is_empty() {
            println!("No deposit receipts found.");
            return Ok(());
        }

        let futs: Vec<_> = receipts
            .iter()
            .map(|(addr, receipt)| calculate_receipt_info(&rpc_client, *addr, receipt))
            .collect();
        let mut receipt_infos: Vec<ReceiptInfo> = futures::future::join_all(futs).await;

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

    async fn claim_tokens(
        &self,
        receipt_address: Pubkey,
        destination: Option<Pubkey>,
        after_cooldown: bool,
        create_ata: bool,
    ) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();
        // Get the receipt data
        let receipt_account = rpc_client.get_account(&receipt_address).await?;

        let receipt =
            stake_deposit_interceptor_program::state::DepositReceipt::try_from_slice_unchecked(
                receipt_account.data.as_slice(),
            )?;

        // Determine after_cooldown automatically: true if fee payer is not the owner
        let auto_after_cooldown = self.cli_config.signer.pubkey() != receipt.owner;
        let _final_after_cooldown = after_cooldown || auto_after_cooldown;

        if auto_after_cooldown && !after_cooldown {
            println!("Note: Setting after_cooldown=true because fee payer ({}) is not the receipt owner ({})",
                     self.cli_config.signer.pubkey(), receipt.owner);
        }

        // Get the stake pool deposit authority
        let authority_account = rpc_client
            .get_account(&receipt.stake_pool_deposit_stake_authority)
            .await?;

        let stake_pool_deposit_authority =
            stake_deposit_interceptor_program::state::StakePoolDepositStakeAuthority::try_from_slice_unchecked(
                authority_account.data.as_slice(),
            )
            ?;

        // Determine the destination token account
        let destination_token_account = match destination {
            Some(dest) => dest,
            None => get_associated_token_address(
                &receipt.owner,
                &stake_pool_deposit_authority.pool_mint,
            ),
        };

        // Get fee wallet token account
        let fee_wallet_token_account = get_associated_token_address(
            &stake_pool_deposit_authority.fee_wallet,
            &stake_pool_deposit_authority.pool_mint,
        );

        // Collect all instructions
        let mut instructions = Vec::new();

        // Check if destination account exists, add creation instruction if needed
        if rpc_client
            .get_account(&destination_token_account)
            .await
            .is_err()
        {
            if create_ata {
                println!("Will create destination token account: {destination_token_account}");

                let create_ata_ix =
                    spl_associated_token_account_interface::instruction::create_associated_token_account(
                        &self.cli_config.signer.pubkey(),
                        &receipt.owner,
                        &stake_pool_deposit_authority.pool_mint,
                        &spl_token::id(),
                    );
                instructions.push(create_ata_ix);
            } else {
                return Err(anyhow!(
                    "Destination token account {destination_token_account} does not exist. Use --create-ata to create it.",
                ));
            }
        }

        // Check if fee account exists, add creation instruction if needed
        if rpc_client
            .get_account(&fee_wallet_token_account)
            .await
            .is_err()
        {
            println!("Will create fee wallet token account: {fee_wallet_token_account}");

            let create_fee_ata_ix =
                spl_associated_token_account_interface::instruction::create_associated_token_account(
                    &self.cli_config.signer.pubkey(),
                    &stake_pool_deposit_authority.fee_wallet,
                    &stake_pool_deposit_authority.pool_mint,
                    &spl_token::id(),
                );
            instructions.push(create_fee_ata_ix);
        }

        let mut ix_builder = ClaimPoolTokensBuilder::new();
        ix_builder
            .deposit_receipt(
                self.stake_deposit_receipt_address(&receipt.stake_pool, &receipt.base)
                    .0,
            )
            .owner(receipt.owner)
            .vault(stake_pool_deposit_authority.vault)
            .destination(destination_token_account)
            .fee_wallet(fee_wallet_token_account)
            .deposit_authority(receipt.stake_pool_deposit_stake_authority)
            .pool_mint(stake_pool_deposit_authority.pool_mint);
        let mut ix = ix_builder.instruction();
        ix.program_id = self.stake_deposit_interceptor_program_id;

        instructions.push(ix);

        log::info!("Depositing Stake parameters: {ix_builder:?}",);

        self.process_transaction(
            &instructions,
            &self.cli_config.signer.pubkey(),
            std::slice::from_ref(&self.cli_config.signer),
        )
        .await?;

        Ok(())
    }

    pub async fn deposit_stake_whitelisted(
        &self,
        whitelist: Pubkey,
        stake_deposit_authority_address: Pubkey,
        deposit_stake: Pubkey,
        validator_stake: Pubkey,
        spl_stake_pool_program_id: Pubkey,
    ) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();

        let stake_deposit_authority_acc = rpc_client
            .get_account(&stake_deposit_authority_address)
            .await?;
        let stake_deposit_authority = stake_deposit_interceptor_program::state::StakePoolDepositStakeAuthority::try_from_slice_unchecked(stake_deposit_authority_acc.data.as_slice())?;

        let stake_pool_acc = rpc_client
            .get_account(&stake_deposit_authority.stake_pool)
            .await?;
        let stake_pool: StakePool = try_from_slice_unchecked(stake_pool_acc.data.as_slice())?;

        let pool_tokens_to =
            get_associated_token_address(&self.cli_config.signer.pubkey(), &stake_pool.pool_mint);

        // Authorize the deposit stake account's staker and withdrawer to the stake deposit authority
        let authorize_staker_ix = solana_stake_interface::instruction::authorize(
            &deposit_stake,
            &self.cli_config.signer.pubkey(),
            &stake_deposit_authority_address,
            solana_stake_interface::state::StakeAuthorize::Staker,
            None,
        );
        let authorize_withdrawer_ix = solana_stake_interface::instruction::authorize(
            &deposit_stake,
            &self.cli_config.signer.pubkey(),
            &stake_deposit_authority_address,
            solana_stake_interface::state::StakeAuthorize::Withdrawer,
            None,
        );

        let mut ix_builder = DepositStakeWhitelistedBuilder::new();
        ix_builder
            .whitelisted_signer(self.cli_config.signer.pubkey())
            .whitelist(whitelist)
            .stake_pool(stake_deposit_authority.stake_pool)
            .validator_list(stake_pool.validator_list)
            .stake_deposit_authority(stake_deposit_authority_address)
            .withdraw_authority(pubkey!("8HPpFV5PFqGmDumjRTFw9BhsjrZYjJBDuHX2p6H5nBmd"))
            .deposit_stake(deposit_stake)
            .validator_stake(validator_stake)
            .reserve_stake(stake_pool.reserve_stake)
            .pool_tokens_to(pool_tokens_to)
            .manager_fee_account(stake_pool.manager_fee_account)
            .referral_fee_account(pool_tokens_to)
            .pool_mint(stake_pool.pool_mint)
            .clock(solana_clock::Clock::id())
            .stake_history(solana_stake_interface::stake_history::StakeHistory::id())
            .spl_stake_pool_program(spl_stake_pool_program_id)
            .stake_program(solana_stake_interface::program::id());
        let mut ix = ix_builder.instruction();
        ix.program_id = self.stake_deposit_interceptor_program_id;

        log::info!("Depositing Stake Whitelisted parameters: {ix_builder:?}",);

        self.process_transaction(
            &[authorize_staker_ix, authorize_withdrawer_ix, ix],
            &self.cli_config.signer.pubkey(),
            std::slice::from_ref(&self.cli_config.signer),
        )
        .await?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw_stake_whitelisted(
        &self,
        whitelist: Pubkey,
        stake_deposit_authority_address: Pubkey,
        stake_split_from: Pubkey,
        stake_split_to_path: &str,
        user_stake_authority: Pubkey,
        fee_rebate_recipient: Pubkey,
        spl_stake_pool_program_id: Pubkey,
        amount: u64,
    ) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();

        let stake_deposit_authority_acc = rpc_client
            .get_account(&stake_deposit_authority_address)
            .await?;
        let stake_deposit_authority = stake_deposit_interceptor_program::state::StakePoolDepositStakeAuthority::try_from_slice_unchecked(stake_deposit_authority_acc.data.as_slice())?;

        let stake_pool_acc = rpc_client
            .get_account(&stake_deposit_authority.stake_pool)
            .await?;
        let stake_pool: StakePool = try_from_slice_unchecked(stake_pool_acc.data.as_slice())?;

        let pool_tokens_to =
            get_associated_token_address(&self.cli_config.signer.pubkey(), &stake_pool.pool_mint);

        // Load the stake_split_to keypair and create the account on-chain
        let stake_split_to_keypair = solana_keypair::read_keypair_file(stake_split_to_path)
            .map_err(|e| anyhow!("Failed to read stake_split_to keypair: {e}"))?;
        let stake_space = std::mem::size_of::<solana_stake_interface::state::StakeStateV2>();
        let stake_rent = rpc_client
            .get_minimum_balance_for_rent_exemption(stake_space)
            .await?;
        let create_stake_account_ix = solana_system_interface::instruction::create_account(
            &self.cli_config.signer.pubkey(),
            &stake_split_to_keypair.pubkey(),
            stake_rent,
            stake_space as u64,
            &solana_stake_interface::program::id(),
        );

        let mut ix_builder = WithdrawStakeWhitelistedBuilder::new();
        ix_builder
            .stake_deposit_authority(stake_deposit_authority_address)
            .whitelisted_signer(self.cli_config.signer.pubkey())
            .whitelist(whitelist)
            .stake_pool(stake_deposit_authority.stake_pool)
            .validator_list(stake_pool.validator_list)
            .withdraw_authority(pubkey!("8HPpFV5PFqGmDumjRTFw9BhsjrZYjJBDuHX2p6H5nBmd"))
            .stake_split_from(stake_split_from)
            .stake_split_to(stake_split_to_keypair.pubkey())
            .user_stake_authority(user_stake_authority)
            .user_transfer_authority(self.cli_config.signer.pubkey())
            .user_pool_token_account(pool_tokens_to)
            .manager_fee_account(stake_pool.manager_fee_account)
            .pool_mint(stake_pool.pool_mint)
            .fee_rebate_hopper(
                Hopper::find_program_address(
                    &self.stake_deposit_interceptor_program_id,
                    &whitelist,
                )
                .0,
            )
            .fee_rebate_recipient(fee_rebate_recipient)
            .clock(solana_clock::Clock::id())
            .stake_program(solana_stake_interface::program::id())
            .spl_stake_pool_program(spl_stake_pool_program_id)
            .amount(amount);
        let mut ix = ix_builder.instruction();
        ix.program_id = self.stake_deposit_interceptor_program_id;

        log::info!("Withdrawing Stake Whitelisted parameters: {ix_builder:?}",);

        self.process_transaction(
            &[create_stake_account_ix, ix],
            &self.cli_config.signer.pubkey(),
            &[
                self.cli_config.signer.clone(),
                Arc::new(stake_split_to_keypair),
            ],
        )
        .await?;

        Ok(())
    }

    pub async fn fund_hopper(&self, whitelist_pda: Pubkey, lamports: u64) -> anyhow::Result<()> {
        let hopper_pda = Hopper::find_program_address(
            &self.stake_deposit_interceptor_program_id,
            &whitelist_pda,
        )
        .0;

        let ix = transfer(&self.cli_config.signer.pubkey(), &hopper_pda, lamports);

        self.process_transaction(
            &[ix],
            &self.cli_config.signer.pubkey(),
            &[self.cli_config.signer.clone()],
        )
        .await?;

        Ok(())
    }

    pub async fn hopper_balance(&self, whitelist_pda: Pubkey) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();
        let hopper_pda = Hopper::find_program_address(
            &self.stake_deposit_interceptor_program_id,
            &whitelist_pda,
        )
        .0;
        let hopper_acc = rpc_client.get_account(&hopper_pda).await?;

        println!("Hopper Balance: {}", hopper_acc.lamports);

        Ok(())
    }

    /// Creates a new RPC client using the configuration from the CLI handler.
    ///
    /// This method constructs an RPC client with the URL and commitment level specified in the
    /// CLI configuration. The client can be used to communicate with a Solana node for
    /// submitting transactions, querying account data, and other RPC operations.
    fn get_rpc_client(&self) -> RpcClient {
        RpcClient::new_with_commitment(self.cli_config.rpc_url.clone(), self.cli_config.commitment)
    }

    // /// Fetches and deserializes an account
    // ///
    // /// This method retrieves account data using the configured RPC client,
    // /// then deserializes it into the specified account type using Borsh deserialization.
    // async fn get_account<T: BorshDeserialize>(&self, account_pubkey: &Pubkey) -> anyhow::Result<T> {
    //     let rpc_client = self.get_rpc_client();

    //     let account = rpc_client.get_account(account_pubkey).await?;
    //     let account = T::deserialize(&mut account.data.as_slice())?;

    //     Ok(account)
    // }

    /// Processes a transaction by either printing it as Base58 or sending it.
    ///
    /// This method handles the logic for processing a set of instructions as a transaction.
    /// If `print_tx` is enabled in the CLI handler (helpful for running commands in Squads), it will print the transaction in Base58 format
    /// without sending it. Otherwise, it will submit and confirm the transaction.
    async fn process_transaction<T>(
        &self,
        ixs: &[Instruction],
        payer: &Pubkey,
        signers: &T,
    ) -> anyhow::Result<()>
    where
        T: Signers + ?Sized,
    {
        let rpc_client = self.get_rpc_client();

        let blockhash = rpc_client.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(ixs, Some(payer), signers, blockhash);
        let result = rpc_client.send_and_confirm_transaction(&tx).await?;

        log::info!("Transaction confirmed: {result:?}");

        Ok(())
    }

    // #[allow(dead_code)]
    // fn print_base58_tx(&self, ixs: &[Instruction]) {
    //     ixs.iter().for_each(|ix| {
    //         log::info!("\n------ IX ------\n");

    //         println!("{}\n", ix.program_id);

    //         ix.accounts.iter().for_each(|account| {
    //             let pubkey = format!("{}", account.pubkey);
    //             let writable = if account.is_writable { "W" } else { "" };
    //             let signer = if account.is_signer { "S" } else { "" };

    //             println!("{:<44} {:>2} {:>1}", pubkey, writable, signer);
    //         });

    //         println!("\n");

    //         let base58_string = bs58::encode(&ix.data).into_string();
    //         println!("{}\n", base58_string);
    //     });
    // }
}
