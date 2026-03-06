mod helpers;

#[cfg(test)]
mod tests {
    use solana_keypair::{Keypair, Signer};
    use solana_program::{native_token::LAMPORTS_PER_SOL, program_pack::Pack};
    use solana_program_test::ProgramTestContext;
    use solana_pubkey::Pubkey;
    use spl_associated_token_account_interface::{
        address::get_associated_token_address, instruction::create_associated_token_account,
    };
    use spl_pod::solana_program::borsh1::try_from_slice_unchecked;
    use stake_deposit_interceptor_client::errors::StakeDepositInterceptorError;
    use stake_deposit_interceptor_program::{
        instruction::derive_stake_pool_deposit_stake_authority,
        state::StakePoolDepositStakeAuthority,
    };

    use crate::helpers::{
        airdrop_lamports, create_stake_account, create_stake_deposit_authority,
        create_token_account, create_validator_and_add_to_pool, delegate_stake_account,
        get_account, get_account_data_deserialized, program_test_context_with_stake_pool_state,
        stake_deposit_interceptor_client::{
            assert_stake_deposit_interceptor_error, StakeDepositInterceptorProgramClient,
        },
        stake_pool_update_all, update_stake_deposit_authority,
        whitelist_management_client::WhitelistManagementProgramClient,
        StakePoolAccounts, ValidatorStakeAccount,
    };

    async fn setup() -> (
        ProgramTestContext,
        StakePoolAccounts,
        spl_stake_pool::state::StakePool,
        ValidatorStakeAccount,
        StakePoolDepositStakeAuthority,
        Keypair,
        Pubkey,
        Keypair,
        Keypair,
        u64,
    ) {
        let (mut ctx, stake_pool_accounts) = program_test_context_with_stake_pool_state().await;
        let rent = ctx.banks_client.get_rent().await.unwrap();
        let stake_pool_account = ctx
            .banks_client
            .get_account(stake_pool_accounts.stake_pool)
            .await
            .unwrap()
            .unwrap();
        let stake_pool =
            try_from_slice_unchecked::<spl_stake_pool::state::StakePool>(&stake_pool_account.data)
                .unwrap();
        let deposit_authority_base = Keypair::new();
        let (deposit_stake_authority_pubkey, _bump) = derive_stake_pool_deposit_stake_authority(
            &stake_deposit_interceptor_program::id(),
            &stake_pool_accounts.stake_pool,
            &deposit_authority_base.pubkey(),
        );
        // Set the StakePool's stake_deposit_authority to the interceptor program's PDA
        update_stake_deposit_authority(
            &mut ctx.banks_client,
            &stake_pool_accounts,
            &deposit_stake_authority_pubkey,
            &ctx.payer,
            ctx.last_blockhash,
        )
        .await;
        // Add a validator to the stake_pool
        let validator_stake_accounts =
            create_validator_and_add_to_pool(&mut ctx, &stake_pool_accounts).await;

        let authority = Keypair::new();
        create_stake_deposit_authority(
            &mut ctx,
            &stake_pool_accounts.stake_pool,
            &stake_pool.pool_mint,
            &authority,
            &deposit_authority_base,
            None,
        )
        .await;

        // Set the jito_whitelist_management_program_id on the StakePoolDepositStakeAuthority
        let update_ix =
        stake_deposit_interceptor_program::instruction::create_update_deposit_stake_authority_instruction(
            &stake_deposit_interceptor_program::id(),
            &stake_pool_accounts.stake_pool,
            &authority.pubkey(),
            &deposit_authority_base.pubkey(),
            None,
            None,
            None,
            None,
            Some(jito_whitelist_management_client::programs::JITO_WHITELIST_MANAGEMENT_ID),
        );
        let tx = solana_transaction::Transaction::new_signed_with_payer(
            &[update_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &authority],
            ctx.last_blockhash,
        );
        ctx.banks_client.process_transaction(tx).await.unwrap();

        let depositor = Keypair::new();
        airdrop_lamports(&mut ctx, &depositor.pubkey(), 10 * LAMPORTS_PER_SOL).await;

        // Create "Depositor" owned stake account
        let authorized = solana_stake_interface::state::Authorized {
            staker: depositor.pubkey(),
            withdrawer: depositor.pubkey(),
        };
        let lockup = solana_stake_interface::state::Lockup::default();
        let stake_amount = 2 * LAMPORTS_PER_SOL;
        let total_staked_amount = rent.minimum_balance(std::mem::size_of::<
            solana_stake_interface::state::StakeStateV2,
        >()) + stake_amount;
        let depositor_stake_account = create_stake_account(
            &mut ctx.banks_client,
            &depositor,
            &authorized,
            &lockup,
            stake_amount,
            ctx.last_blockhash,
        )
        .await;

        // Create a TokenAccount for the "Depositor" of the StakePool's `pool_mint`.
        let _depositor_lst_account = create_token_account(
            &mut ctx,
            &depositor.pubkey(),
            &stake_pool_accounts.pool_mint,
        )
        .await;

        // Delegate the "Depositor" stake account to a validator from
        // the relevant StakePool.
        delegate_stake_account(
            &mut ctx.banks_client,
            &depositor,
            &ctx.last_blockhash,
            &depositor_stake_account,
            &depositor,
            &validator_stake_accounts.vote.pubkey(),
        )
        .await;

        // Fast forward to next epoch so stake is active
        let first_normal_slot = ctx.genesis_config().epoch_schedule.first_normal_slot;
        ctx.warp_to_slot(first_normal_slot + 1).unwrap();

        // Update relevant stake_pool state
        stake_pool_update_all(
            &mut ctx.banks_client,
            &ctx.payer,
            &stake_pool_accounts,
            &ctx.last_blockhash,
            false,
        )
        .await;

        // Get latest `StakePoolDepositStakeAuthority``
        let deposit_stake_authority =
            get_account_data_deserialized::<StakePoolDepositStakeAuthority>(
                &mut ctx.banks_client,
                &deposit_stake_authority_pubkey,
            )
            .await;

        // Generate a random Pubkey as seed for DepositReceipt PDA.
        let deposit_receipt_base = Keypair::new();
        (
            ctx,
            stake_pool_accounts,
            stake_pool,
            validator_stake_accounts,
            deposit_stake_authority,
            depositor,
            depositor_stake_account,
            deposit_receipt_base,
            deposit_authority_base,
            total_staked_amount,
        )
    }

    #[tokio::test]
    async fn test_deposit_stake_whitelisted_ok() {
        let (
            mut ctx,
            stake_pool_accounts,
            stake_pool,
            validator_stake_accounts,
            deposit_stake_authority,
            depositor,
            depositor_stake_account,
            _deposit_receipt_base,
            deposit_authority_base,
            total_staked_amount,
        ) = setup().await;

        // Build clients from the same context that has all accounts
        let mut whitelist_management_program_client = WhitelistManagementProgramClient::new(
            ctx.banks_client.clone(),
            ctx.payer.insecure_clone(),
        );
        let mut stake_deposit_interceptor_program_client =
            StakeDepositInterceptorProgramClient::new(
                ctx.banks_client.clone(),
                ctx.payer.insecure_clone(),
            );

        let admin = Keypair::new();
        airdrop_lamports(&mut ctx, &admin.pubkey(), LAMPORTS_PER_SOL).await;

        whitelist_management_program_client
            .do_initialize_whitelist(admin.pubkey())
            .await;

        let whitelisted_signer = Keypair::new();
        airdrop_lamports(&mut ctx, &whitelisted_signer.pubkey(), LAMPORTS_PER_SOL).await;

        whitelist_management_program_client
            .do_add_to_whitelist(&admin, whitelisted_signer.pubkey())
            .await;

        let (deposit_stake_authority_pubkey, _bump_seed) =
            derive_stake_pool_deposit_stake_authority(
                &stake_deposit_interceptor_program::id(),
                &stake_pool_accounts.stake_pool,
                &deposit_authority_base.pubkey(),
            );

        // Create the pool token ATA for the whitelisted signer
        let pool_tokens_to = get_associated_token_address(
            &whitelisted_signer.pubkey(),
            &stake_pool_accounts.pool_mint,
        );
        let create_ata_ix = create_associated_token_account(
            &ctx.payer.pubkey(),
            &whitelisted_signer.pubkey(),
            &stake_pool_accounts.pool_mint,
            &spl_token_interface::id(),
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let create_ata_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(create_ata_tx)
            .await
            .unwrap();

        // Authorize the depositor's stake account staker & withdrawer to the interceptor PDA
        // (same as what the normal DepositStake path does client-side)
        let authorize_staker_ix = solana_stake_interface::instruction::authorize(
            &depositor_stake_account,
            &depositor.pubkey(),
            &deposit_stake_authority_pubkey,
            solana_stake_interface::state::StakeAuthorize::Staker,
            None,
        );
        let authorize_withdrawer_ix = solana_stake_interface::instruction::authorize(
            &depositor_stake_account,
            &depositor.pubkey(),
            &deposit_stake_authority_pubkey,
            solana_stake_interface::state::StakeAuthorize::Withdrawer,
            None,
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let authorize_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[authorize_staker_ix, authorize_withdrawer_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &depositor],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(authorize_tx)
            .await
            .unwrap();

        stake_deposit_interceptor_program_client
            .deposit_stake_whitelisted(
                &whitelisted_signer,
                whitelist_management_program_client.get_whitelist_pda(),
                stake_pool_accounts.stake_pool,
                stake_pool_accounts.validator_list,
                deposit_stake_authority_pubkey,
                stake_pool_accounts.withdraw_authority,
                depositor_stake_account,
                validator_stake_accounts.stake_account,
                stake_pool.reserve_stake,
                pool_tokens_to,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_mint,
                spl_stake_pool::id(),
            )
            .await
            .unwrap();

        let pool_tokens_amount = spl_stake_pool::state::StakePool::calc_pool_tokens_for_deposit(
            &stake_pool,
            total_staked_amount,
        )
        .unwrap();

        // Assert LST was minted directly to the whitelisted signer's token account (no vault/ticket)
        let pool_tokens_to_account = get_account(&mut ctx.banks_client, &pool_tokens_to).await;
        let pool_tokens_to_token =
            spl_token_interface::state::Account::unpack(&pool_tokens_to_account.data).unwrap();
        assert_eq!(pool_tokens_to_token.amount, pool_tokens_amount);

        // Assert the vault is empty (no tokens held by the interceptor)
        let vault_account =
            get_account(&mut ctx.banks_client, &deposit_stake_authority.vault).await;
        let vault = spl_token_interface::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault.amount, 0);
    }

    #[tokio::test]
    async fn test_deposit_stake_whitelisted_invalid_whitelisted_signer_fails() {
        let (
            mut ctx,
            stake_pool_accounts,
            stake_pool,
            validator_stake_accounts,
            _deposit_stake_authority,
            depositor,
            depositor_stake_account,
            _deposit_receipt_base,
            deposit_authority_base,
            _total_staked_amount,
        ) = setup().await;

        // Build clients from the same context that has all accounts
        let mut whitelist_management_program_client = WhitelistManagementProgramClient::new(
            ctx.banks_client.clone(),
            ctx.payer.insecure_clone(),
        );
        let mut stake_deposit_interceptor_program_client =
            StakeDepositInterceptorProgramClient::new(
                ctx.banks_client.clone(),
                ctx.payer.insecure_clone(),
            );

        let admin = Keypair::new();
        airdrop_lamports(&mut ctx, &admin.pubkey(), LAMPORTS_PER_SOL).await;

        whitelist_management_program_client
            .do_initialize_whitelist(admin.pubkey())
            .await;

        let whitelisted_signer = Keypair::new();
        airdrop_lamports(&mut ctx, &whitelisted_signer.pubkey(), LAMPORTS_PER_SOL).await;

        whitelist_management_program_client
            .do_add_to_whitelist(&admin, whitelisted_signer.pubkey())
            .await;

        let (deposit_stake_authority_pubkey, _bump_seed) =
            derive_stake_pool_deposit_stake_authority(
                &stake_deposit_interceptor_program::id(),
                &stake_pool_accounts.stake_pool,
                &deposit_authority_base.pubkey(),
            );

        // Create the pool token ATA for the whitelisted signer
        let pool_tokens_to = get_associated_token_address(
            &whitelisted_signer.pubkey(),
            &stake_pool_accounts.pool_mint,
        );
        let create_ata_ix = create_associated_token_account(
            &ctx.payer.pubkey(),
            &whitelisted_signer.pubkey(),
            &stake_pool_accounts.pool_mint,
            &spl_token_interface::id(),
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let create_ata_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(create_ata_tx)
            .await
            .unwrap();

        // Authorize the depositor's stake account staker & withdrawer to the interceptor PDA
        // (same as what the normal DepositStake path does client-side)
        let authorize_staker_ix = solana_stake_interface::instruction::authorize(
            &depositor_stake_account,
            &depositor.pubkey(),
            &deposit_stake_authority_pubkey,
            solana_stake_interface::state::StakeAuthorize::Staker,
            None,
        );
        let authorize_withdrawer_ix = solana_stake_interface::instruction::authorize(
            &depositor_stake_account,
            &depositor.pubkey(),
            &deposit_stake_authority_pubkey,
            solana_stake_interface::state::StakeAuthorize::Withdrawer,
            None,
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let authorize_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[authorize_staker_ix, authorize_withdrawer_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &depositor],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(authorize_tx)
            .await
            .unwrap();

        let invalid_whitelisted_signer = Keypair::new();
        airdrop_lamports(
            &mut ctx,
            &invalid_whitelisted_signer.pubkey(),
            LAMPORTS_PER_SOL,
        )
        .await;

        let test_error = stake_deposit_interceptor_program_client
            .deposit_stake_whitelisted(
                &invalid_whitelisted_signer,
                whitelist_management_program_client.get_whitelist_pda(),
                stake_pool_accounts.stake_pool,
                stake_pool_accounts.validator_list,
                deposit_stake_authority_pubkey,
                stake_pool_accounts.withdraw_authority,
                depositor_stake_account,
                validator_stake_accounts.stake_account,
                stake_pool.reserve_stake,
                pool_tokens_to,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_mint,
                spl_stake_pool::id(),
            )
            .await;

        assert_stake_deposit_interceptor_error(
            test_error,
            StakeDepositInterceptorError::InvalidWhitelistedSigner,
        );
    }

    #[tokio::test]
    async fn test_deposit_stake_whitelisted_invalid_spl_stake_pool_program_id_fails() {
        let (
            mut ctx,
            stake_pool_accounts,
            stake_pool,
            validator_stake_accounts,
            _deposit_stake_authority,
            depositor,
            depositor_stake_account,
            _deposit_receipt_base,
            deposit_authority_base,
            _total_staked_amount,
        ) = setup().await;

        // Build clients from the same context that has all accounts
        let mut whitelist_management_program_client = WhitelistManagementProgramClient::new(
            ctx.banks_client.clone(),
            ctx.payer.insecure_clone(),
        );
        let mut stake_deposit_interceptor_program_client =
            StakeDepositInterceptorProgramClient::new(
                ctx.banks_client.clone(),
                ctx.payer.insecure_clone(),
            );

        let admin = Keypair::new();
        airdrop_lamports(&mut ctx, &admin.pubkey(), LAMPORTS_PER_SOL).await;

        whitelist_management_program_client
            .do_initialize_whitelist(admin.pubkey())
            .await;

        let whitelisted_signer = Keypair::new();
        airdrop_lamports(&mut ctx, &whitelisted_signer.pubkey(), LAMPORTS_PER_SOL).await;

        whitelist_management_program_client
            .do_add_to_whitelist(&admin, whitelisted_signer.pubkey())
            .await;

        let (deposit_stake_authority_pubkey, _bump_seed) =
            derive_stake_pool_deposit_stake_authority(
                &stake_deposit_interceptor_program::id(),
                &stake_pool_accounts.stake_pool,
                &deposit_authority_base.pubkey(),
            );

        // Create the pool token ATA for the whitelisted signer
        let pool_tokens_to = get_associated_token_address(
            &whitelisted_signer.pubkey(),
            &stake_pool_accounts.pool_mint,
        );
        let create_ata_ix = create_associated_token_account(
            &ctx.payer.pubkey(),
            &whitelisted_signer.pubkey(),
            &stake_pool_accounts.pool_mint,
            &spl_token_interface::id(),
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let create_ata_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(create_ata_tx)
            .await
            .unwrap();

        // Authorize the depositor's stake account staker & withdrawer to the interceptor PDA
        // (same as what the normal DepositStake path does client-side)
        let authorize_staker_ix = solana_stake_interface::instruction::authorize(
            &depositor_stake_account,
            &depositor.pubkey(),
            &deposit_stake_authority_pubkey,
            solana_stake_interface::state::StakeAuthorize::Staker,
            None,
        );
        let authorize_withdrawer_ix = solana_stake_interface::instruction::authorize(
            &depositor_stake_account,
            &depositor.pubkey(),
            &deposit_stake_authority_pubkey,
            solana_stake_interface::state::StakeAuthorize::Withdrawer,
            None,
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let authorize_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[authorize_staker_ix, authorize_withdrawer_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &depositor],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(authorize_tx)
            .await
            .unwrap();

        let invalid_spl_stake_pool_program_id = Pubkey::new_unique();

        let test_error = stake_deposit_interceptor_program_client
            .deposit_stake_whitelisted(
                &whitelisted_signer,
                whitelist_management_program_client.get_whitelist_pda(),
                stake_pool_accounts.stake_pool,
                stake_pool_accounts.validator_list,
                deposit_stake_authority_pubkey,
                stake_pool_accounts.withdraw_authority,
                depositor_stake_account,
                validator_stake_accounts.stake_account,
                stake_pool.reserve_stake,
                pool_tokens_to,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_mint,
                invalid_spl_stake_pool_program_id,
            )
            .await;

        assert_stake_deposit_interceptor_error(
            test_error,
            StakeDepositInterceptorError::InvalidStakePoolProgram,
        );
    }
}
