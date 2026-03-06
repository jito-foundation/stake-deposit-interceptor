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
        stake_client::StakeProgramClient,
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
    async fn test_withdraw_stake_whitelisted_ok() {
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
        let mut stake_program_client =
            StakeProgramClient::new(ctx.banks_client.clone(), ctx.payer.insecure_clone());
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
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Staker,
            )
            .await
            .unwrap();
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Withdrawer,
            )
            .await
            .unwrap();

        let whitelist_pda = whitelist_management_program_client.get_whitelist_pda();

        stake_deposit_interceptor_program_client
            .deposit_stake_whitelisted(
                &whitelisted_signer,
                whitelist_pda,
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

        let pool_tokens_to_account = get_account(&mut ctx.banks_client, &pool_tokens_to).await;
        let pool_tokens_to_token =
            spl_token_interface::state::Account::unpack(&pool_tokens_to_account.data).unwrap();

        // Create an uninitialized stake account for the split destination
        let stake_split_to = Keypair::new();
        let rent = ctx.banks_client.get_rent().await.unwrap();
        let stake_account_rent = rent.minimum_balance(std::mem::size_of::<
            solana_stake_interface::state::StakeStateV2,
        >());
        let create_split_to_ix = solana_system_interface::instruction::create_account(
            &ctx.payer.pubkey(),
            &stake_split_to.pubkey(),
            stake_account_rent,
            std::mem::size_of::<solana_stake_interface::state::StakeStateV2>() as u64,
            &solana_stake_interface::program::id(),
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let create_split_to_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[create_split_to_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &stake_split_to],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(create_split_to_tx)
            .await
            .unwrap();

        let user_stake_authority = Pubkey::new_unique();
        let user_transfer_authority = whitelisted_signer.insecure_clone();
        let fee_rebate_receiver = Pubkey::new_unique();

        let hopper_pda = stake_deposit_interceptor_program_client.get_hopper_pda(&whitelist_pda);
        airdrop_lamports(&mut ctx, &hopper_pda, LAMPORTS_PER_SOL).await;

        stake_deposit_interceptor_program_client
            .withdraw_stake_whitelisted(
                deposit_stake_authority_pubkey,
                whitelisted_signer,
                whitelist_pda,
                stake_pool_accounts.stake_pool,
                stake_pool_accounts.validator_list,
                stake_pool_accounts.withdraw_authority,
                validator_stake_accounts.stake_account,
                stake_split_to.pubkey(),
                user_stake_authority,
                user_transfer_authority,
                pool_tokens_to,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_mint,
                hopper_pda,
                fee_rebate_receiver,
                spl_stake_pool::id(),
                pool_tokens_to_token.amount,
            )
            .await
            .unwrap();

        // Assert all pool tokens were burned
        let pool_tokens_to_account_after =
            get_account(&mut ctx.banks_client, &pool_tokens_to).await;
        let pool_tokens_to_token_after =
            spl_token_interface::state::Account::unpack(&pool_tokens_to_account_after.data)
                .unwrap();
        assert_eq!(pool_tokens_to_token_after.amount, 0);

        // Assert the split-to stake account received stake
        let split_to_account = get_account(&mut ctx.banks_client, &stake_split_to.pubkey()).await;
        assert!(split_to_account.lamports > stake_account_rent);

        // Assert the fee rebate was transferred from the hopper to the recipient
        let hopper_account_after = get_account(&mut ctx.banks_client, &hopper_pda).await;
        assert!(hopper_account_after.lamports < LAMPORTS_PER_SOL);

        let fee_rebate_receiver_account =
            get_account(&mut ctx.banks_client, &fee_rebate_receiver).await;
        assert!(fee_rebate_receiver_account.lamports > 0);
    }

    #[tokio::test]
    async fn test_withdraw_stake_whitelisted_fee_receiver_non_system_account() {
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
        let mut stake_program_client =
            StakeProgramClient::new(ctx.banks_client.clone(), ctx.payer.insecure_clone());
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
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Staker,
            )
            .await
            .unwrap();
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Withdrawer,
            )
            .await
            .unwrap();

        let whitelist_pda = whitelist_management_program_client.get_whitelist_pda();

        stake_deposit_interceptor_program_client
            .deposit_stake_whitelisted(
                &whitelisted_signer,
                whitelist_pda,
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

        let pool_tokens_to_account = get_account(&mut ctx.banks_client, &pool_tokens_to).await;
        let pool_tokens_to_token =
            spl_token_interface::state::Account::unpack(&pool_tokens_to_account.data).unwrap();

        // Create an uninitialized stake account for the split destination
        let stake_split_to = Keypair::new();
        let rent = ctx.banks_client.get_rent().await.unwrap();
        let stake_account_rent = rent.minimum_balance(std::mem::size_of::<
            solana_stake_interface::state::StakeStateV2,
        >());
        let create_split_to_ix = solana_system_interface::instruction::create_account(
            &ctx.payer.pubkey(),
            &stake_split_to.pubkey(),
            stake_account_rent,
            std::mem::size_of::<solana_stake_interface::state::StakeStateV2>() as u64,
            &solana_stake_interface::program::id(),
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let create_split_to_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[create_split_to_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &stake_split_to],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(create_split_to_tx)
            .await
            .unwrap();

        let user_stake_authority = Pubkey::new_unique();
        let user_transfer_authority = whitelisted_signer.insecure_clone();

        // Create fee_rebate_receiver as an account owned by a non-system program
        // (e.g., a token account owned by the SPL Token program)
        let fee_rebate_receiver_keypair = Keypair::new();
        let fee_rebate_receiver = fee_rebate_receiver_keypair.pubkey();
        let token_account_size = spl_token_interface::state::Account::LEN;
        let token_account_rent = rent.minimum_balance(token_account_size);
        let create_non_system_account_ix = solana_system_interface::instruction::create_account(
            &ctx.payer.pubkey(),
            &fee_rebate_receiver,
            token_account_rent,
            token_account_size as u64,
            &spl_token_interface::id(),
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let create_non_system_account_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[create_non_system_account_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &fee_rebate_receiver_keypair],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(create_non_system_account_tx)
            .await
            .unwrap();

        let hopper_pda = stake_deposit_interceptor_program_client.get_hopper_pda(&whitelist_pda);
        airdrop_lamports(&mut ctx, &hopper_pda, LAMPORTS_PER_SOL).await;

        // Withdraw should succeed even with a non-system-owned fee_rebate_receiver
        stake_deposit_interceptor_program_client
            .withdraw_stake_whitelisted(
                deposit_stake_authority_pubkey,
                whitelisted_signer,
                whitelist_pda,
                stake_pool_accounts.stake_pool,
                stake_pool_accounts.validator_list,
                stake_pool_accounts.withdraw_authority,
                validator_stake_accounts.stake_account,
                stake_split_to.pubkey(),
                user_stake_authority,
                user_transfer_authority,
                pool_tokens_to,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_mint,
                hopper_pda,
                fee_rebate_receiver,
                spl_stake_pool::id(),
                pool_tokens_to_token.amount,
            )
            .await
            .unwrap();

        // Assert the fee rebate was transferred from the hopper to the non-system recipient
        let hopper_account_after = get_account(&mut ctx.banks_client, &hopper_pda).await;
        assert!(hopper_account_after.lamports < LAMPORTS_PER_SOL);

        let fee_rebate_receiver_account =
            get_account(&mut ctx.banks_client, &fee_rebate_receiver).await;
        assert!(fee_rebate_receiver_account.lamports > token_account_rent);
    }

    #[tokio::test]
    async fn test_withdraw_stake_whitelisted_invalid_whitelisted_signer_fails() {
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
        let mut stake_program_client =
            StakeProgramClient::new(ctx.banks_client.clone(), ctx.payer.insecure_clone());
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
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Staker,
            )
            .await
            .unwrap();
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Withdrawer,
            )
            .await
            .unwrap();

        let whitelist_pda = whitelist_management_program_client.get_whitelist_pda();

        stake_deposit_interceptor_program_client
            .deposit_stake_whitelisted(
                &whitelisted_signer,
                whitelist_pda,
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

        let invalid_whitelisted_signer = Keypair::new();
        airdrop_lamports(
            &mut ctx,
            &invalid_whitelisted_signer.pubkey(),
            LAMPORTS_PER_SOL,
        )
        .await;

        let pool_tokens_to_account = get_account(&mut ctx.banks_client, &pool_tokens_to).await;
        let pool_tokens_to_token =
            spl_token_interface::state::Account::unpack(&pool_tokens_to_account.data).unwrap();

        // Create an uninitialized stake account for the split destination
        let stake_split_to = Keypair::new();
        let rent = ctx.banks_client.get_rent().await.unwrap();
        let stake_account_rent = rent.minimum_balance(std::mem::size_of::<
            solana_stake_interface::state::StakeStateV2,
        >());
        let create_split_to_ix = solana_system_interface::instruction::create_account(
            &ctx.payer.pubkey(),
            &stake_split_to.pubkey(),
            stake_account_rent,
            std::mem::size_of::<solana_stake_interface::state::StakeStateV2>() as u64,
            &solana_stake_interface::program::id(),
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let create_split_to_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[create_split_to_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &stake_split_to],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(create_split_to_tx)
            .await
            .unwrap();

        let user_stake_authority = Pubkey::new_unique();
        let user_transfer_authority = whitelisted_signer.insecure_clone();
        let fee_rebate_receiver = Pubkey::new_unique();

        let hopper_pda = stake_deposit_interceptor_program_client.get_hopper_pda(&whitelist_pda);
        airdrop_lamports(&mut ctx, &hopper_pda, LAMPORTS_PER_SOL).await;

        let test_error = stake_deposit_interceptor_program_client
            .withdraw_stake_whitelisted(
                deposit_stake_authority_pubkey,
                invalid_whitelisted_signer,
                whitelist_pda,
                stake_pool_accounts.stake_pool,
                stake_pool_accounts.validator_list,
                stake_pool_accounts.withdraw_authority,
                validator_stake_accounts.stake_account,
                stake_split_to.pubkey(),
                user_stake_authority,
                user_transfer_authority,
                pool_tokens_to,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_mint,
                hopper_pda,
                fee_rebate_receiver,
                spl_stake_pool::id(),
                pool_tokens_to_token.amount,
            )
            .await;

        assert_stake_deposit_interceptor_error(
            test_error,
            StakeDepositInterceptorError::InvalidWhitelistedSigner,
        );
    }

    #[tokio::test]
    async fn test_withdraw_stake_whitelisted_invalid_stake_deposit_authority_fails() {
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
        let mut stake_program_client =
            StakeProgramClient::new(ctx.banks_client.clone(), ctx.payer.insecure_clone());
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
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Staker,
            )
            .await
            .unwrap();
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Withdrawer,
            )
            .await
            .unwrap();

        let whitelist_pda = whitelist_management_program_client.get_whitelist_pda();

        stake_deposit_interceptor_program_client
            .deposit_stake_whitelisted(
                &whitelisted_signer,
                whitelist_pda,
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

        let pool_tokens_to_account = get_account(&mut ctx.banks_client, &pool_tokens_to).await;
        let pool_tokens_to_token =
            spl_token_interface::state::Account::unpack(&pool_tokens_to_account.data).unwrap();

        // Create an uninitialized stake account for the split destination
        let stake_split_to = Keypair::new();
        let rent = ctx.banks_client.get_rent().await.unwrap();
        let stake_account_rent = rent.minimum_balance(std::mem::size_of::<
            solana_stake_interface::state::StakeStateV2,
        >());
        let create_split_to_ix = solana_system_interface::instruction::create_account(
            &ctx.payer.pubkey(),
            &stake_split_to.pubkey(),
            stake_account_rent,
            std::mem::size_of::<solana_stake_interface::state::StakeStateV2>() as u64,
            &solana_stake_interface::program::id(),
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let create_split_to_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[create_split_to_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &stake_split_to],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(create_split_to_tx)
            .await
            .unwrap();

        let user_stake_authority = Pubkey::new_unique();
        let user_transfer_authority = whitelisted_signer.insecure_clone();
        let fee_rebate_receiver = Pubkey::new_unique();

        let hopper_pda = stake_deposit_interceptor_program_client.get_hopper_pda(&whitelist_pda);
        airdrop_lamports(&mut ctx, &hopper_pda, LAMPORTS_PER_SOL).await;

        // Change the stake pool's stake_deposit_authority to a different address
        // so that our PDA no longer matches the stake pool's stake_deposit_authority
        let different_authority = Pubkey::new_unique();
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        update_stake_deposit_authority(
            &mut ctx.banks_client,
            &stake_pool_accounts,
            &different_authority,
            &ctx.payer.insecure_clone(),
            blockhash,
        )
        .await;

        let test_error = stake_deposit_interceptor_program_client
            .withdraw_stake_whitelisted(
                deposit_stake_authority_pubkey,
                whitelisted_signer,
                whitelist_pda,
                stake_pool_accounts.stake_pool,
                stake_pool_accounts.validator_list,
                stake_pool_accounts.withdraw_authority,
                validator_stake_accounts.stake_account,
                stake_split_to.pubkey(),
                user_stake_authority,
                user_transfer_authority,
                pool_tokens_to,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_mint,
                hopper_pda,
                fee_rebate_receiver,
                spl_stake_pool::id(),
                pool_tokens_to_token.amount,
            )
            .await;

        assert_stake_deposit_interceptor_error(
            test_error,
            StakeDepositInterceptorError::InvalidStakePoolDepositStakeAuthority,
        );
    }

    #[tokio::test]
    async fn test_withdraw_stake_whitelisted_invalid_spl_stake_pool_program_id_fails() {
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
        let mut stake_program_client =
            StakeProgramClient::new(ctx.banks_client.clone(), ctx.payer.insecure_clone());
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
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Staker,
            )
            .await
            .unwrap();
        stake_program_client
            .authorize(
                &depositor_stake_account,
                &depositor,
                &deposit_stake_authority_pubkey,
                solana_stake_interface::state::StakeAuthorize::Withdrawer,
            )
            .await
            .unwrap();

        let whitelist_pda = whitelist_management_program_client.get_whitelist_pda();

        stake_deposit_interceptor_program_client
            .deposit_stake_whitelisted(
                &whitelisted_signer,
                whitelist_pda,
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

        let pool_tokens_to_account = get_account(&mut ctx.banks_client, &pool_tokens_to).await;
        let pool_tokens_to_token =
            spl_token_interface::state::Account::unpack(&pool_tokens_to_account.data).unwrap();

        // Create an uninitialized stake account for the split destination
        let stake_split_to = Keypair::new();
        let rent = ctx.banks_client.get_rent().await.unwrap();
        let stake_account_rent = rent.minimum_balance(std::mem::size_of::<
            solana_stake_interface::state::StakeStateV2,
        >());
        let create_split_to_ix = solana_system_interface::instruction::create_account(
            &ctx.payer.pubkey(),
            &stake_split_to.pubkey(),
            stake_account_rent,
            std::mem::size_of::<solana_stake_interface::state::StakeStateV2>() as u64,
            &solana_stake_interface::program::id(),
        );
        let blockhash = ctx.banks_client.get_latest_blockhash().await.unwrap();
        let create_split_to_tx = solana_transaction::Transaction::new_signed_with_payer(
            &[create_split_to_ix],
            Some(&ctx.payer.pubkey()),
            &[&ctx.payer, &stake_split_to],
            blockhash,
        );
        ctx.banks_client
            .process_transaction(create_split_to_tx)
            .await
            .unwrap();

        let user_stake_authority = Pubkey::new_unique();
        let user_transfer_authority = whitelisted_signer.insecure_clone();
        let fee_rebate_receiver = Pubkey::new_unique();

        let hopper_pda = stake_deposit_interceptor_program_client.get_hopper_pda(&whitelist_pda);
        airdrop_lamports(&mut ctx, &hopper_pda, LAMPORTS_PER_SOL).await;

        let invalid_spl_stake_pool_program_id = Pubkey::new_unique();

        let test_error = stake_deposit_interceptor_program_client
            .withdraw_stake_whitelisted(
                deposit_stake_authority_pubkey,
                whitelisted_signer,
                whitelist_pda,
                stake_pool_accounts.stake_pool,
                stake_pool_accounts.validator_list,
                stake_pool_accounts.withdraw_authority,
                validator_stake_accounts.stake_account,
                stake_split_to.pubkey(),
                user_stake_authority,
                user_transfer_authority,
                pool_tokens_to,
                stake_pool_accounts.pool_fee_account,
                stake_pool_accounts.pool_mint,
                hopper_pda,
                fee_rebate_receiver,
                invalid_spl_stake_pool_program_id,
                pool_tokens_to_token.amount,
            )
            .await;

        assert_stake_deposit_interceptor_error(
            test_error,
            StakeDepositInterceptorError::InvalidStakePoolProgram,
        );
    }
}
