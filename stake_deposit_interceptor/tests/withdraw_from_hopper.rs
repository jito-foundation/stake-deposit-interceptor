mod helpers;

#[cfg(test)]
mod tests {
    use solana_keypair::{Keypair, Signer};
    use solana_program::native_token::LAMPORTS_PER_SOL;
    use solana_program_test::ProgramTestContext;
    use spl_pod::solana_program::borsh1::try_from_slice_unchecked;
    use stake_deposit_interceptor_client::errors::StakeDepositInterceptorError;
    use stake_deposit_interceptor_program::{
        instruction::derive_stake_pool_deposit_stake_authority,
        state::StakePoolDepositStakeAuthority,
    };

    use crate::helpers::{
        airdrop_lamports, create_stake_deposit_authority, get_account,
        program_test_context_with_stake_pool_state,
        stake_deposit_interceptor_client::{
            assert_stake_deposit_interceptor_error, StakeDepositInterceptorProgramClient,
        },
        update_stake_deposit_authority,
        whitelist_management_client::WhitelistManagementProgramClient,
        StakePoolAccounts,
    };

    /// Setup: creates a stake pool, deposit stake authority, and whitelist.
    /// Returns the context, accounts, authority keypair, deposit authority base,
    /// deposit stake authority PDA, and whitelist PDA.
    async fn setup() -> (
        ProgramTestContext,
        StakePoolAccounts,
        Keypair,         // authority
        Keypair,         // deposit_authority_base
        StakePoolDepositStakeAuthority,
    ) {
        let (mut ctx, stake_pool_accounts) = program_test_context_with_stake_pool_state().await;
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

        // Get latest StakePoolDepositStakeAuthority
        let deposit_stake_authority: StakePoolDepositStakeAuthority =
            crate::helpers::get_account_data_deserialized(
                &mut ctx.banks_client,
                &deposit_stake_authority_pubkey,
            )
            .await;

        (
            ctx,
            stake_pool_accounts,
            authority,
            deposit_authority_base,
            deposit_stake_authority,
        )
    }

    #[tokio::test]
    async fn test_withdraw_from_hopper_ok() {
        let (mut ctx, _stake_pool_accounts, authority, deposit_authority_base, deposit_stake_authority) =
            setup().await;

        // Initialize whitelist
        let mut whitelist_management_program_client = WhitelistManagementProgramClient::new(
            ctx.banks_client.clone(),
            ctx.payer.insecure_clone(),
        );
        let admin = Keypair::new();
        airdrop_lamports(&mut ctx, &admin.pubkey(), LAMPORTS_PER_SOL).await;
        whitelist_management_program_client
            .do_initialize_whitelist(admin.pubkey())
            .await;
        let whitelist_pda = whitelist_management_program_client.get_whitelist_pda();

        // Get hopper PDA and fund it
        let mut stake_deposit_interceptor_program_client =
            StakeDepositInterceptorProgramClient::new(
                ctx.banks_client.clone(),
                ctx.payer.insecure_clone(),
            );
        let hopper_pda = stake_deposit_interceptor_program_client.get_hopper_pda(&whitelist_pda);
        let hopper_fund_amount = 5 * LAMPORTS_PER_SOL;
        airdrop_lamports(&mut ctx, &hopper_pda, hopper_fund_amount).await;

        // Verify hopper has funds
        let hopper_account_before = get_account(&mut ctx.banks_client, &hopper_pda).await;
        assert_eq!(hopper_account_before.lamports, hopper_fund_amount);

        // Create recipient
        let recipient = Keypair::new();
        let recipient_before = ctx.banks_client.get_account(recipient.pubkey()).await.unwrap();
        assert!(recipient_before.is_none());

        // Derive deposit stake authority PDA
        let (deposit_stake_authority_pubkey, _) = derive_stake_pool_deposit_stake_authority(
            &stake_deposit_interceptor_program::id(),
            &deposit_stake_authority.stake_pool,
            &deposit_authority_base.pubkey(),
        );

        // Withdraw from hopper
        let withdraw_amount = 2 * LAMPORTS_PER_SOL;
        stake_deposit_interceptor_program_client
            .withdraw_from_hopper(
                &authority,
                deposit_stake_authority_pubkey,
                whitelist_pda,
                hopper_pda,
                recipient.pubkey(),
                withdraw_amount,
            )
            .await
            .unwrap();

        // Verify hopper balance decreased
        let hopper_account_after = get_account(&mut ctx.banks_client, &hopper_pda).await;
        assert_eq!(
            hopper_account_after.lamports,
            hopper_fund_amount - withdraw_amount
        );

        // Verify recipient received the funds
        let recipient_account = get_account(&mut ctx.banks_client, &recipient.pubkey()).await;
        assert_eq!(recipient_account.lamports, withdraw_amount);
    }

    #[tokio::test]
    async fn test_withdraw_from_hopper_invalid_authority_fails() {
        let (mut ctx, _stake_pool_accounts, _authority, deposit_authority_base, deposit_stake_authority) =
            setup().await;

        // Initialize whitelist
        let mut whitelist_management_program_client = WhitelistManagementProgramClient::new(
            ctx.banks_client.clone(),
            ctx.payer.insecure_clone(),
        );
        let admin = Keypair::new();
        airdrop_lamports(&mut ctx, &admin.pubkey(), LAMPORTS_PER_SOL).await;
        whitelist_management_program_client
            .do_initialize_whitelist(admin.pubkey())
            .await;
        let whitelist_pda = whitelist_management_program_client.get_whitelist_pda();

        // Get hopper PDA and fund it
        let mut stake_deposit_interceptor_program_client =
            StakeDepositInterceptorProgramClient::new(
                ctx.banks_client.clone(),
                ctx.payer.insecure_clone(),
            );
        let hopper_pda = stake_deposit_interceptor_program_client.get_hopper_pda(&whitelist_pda);
        airdrop_lamports(&mut ctx, &hopper_pda, 5 * LAMPORTS_PER_SOL).await;

        let (deposit_stake_authority_pubkey, _) = derive_stake_pool_deposit_stake_authority(
            &stake_deposit_interceptor_program::id(),
            &deposit_stake_authority.stake_pool,
            &deposit_authority_base.pubkey(),
        );

        // Use a wrong authority
        let wrong_authority = Keypair::new();
        airdrop_lamports(&mut ctx, &wrong_authority.pubkey(), LAMPORTS_PER_SOL).await;

        let recipient = Keypair::new();
        let result = stake_deposit_interceptor_program_client
            .withdraw_from_hopper(
                &wrong_authority,
                deposit_stake_authority_pubkey,
                whitelist_pda,
                hopper_pda,
                recipient.pubkey(),
                LAMPORTS_PER_SOL,
            )
            .await;

        assert_stake_deposit_interceptor_error(
            result,
            StakeDepositInterceptorError::InvalidAuthority,
        );
    }

    #[tokio::test]
    async fn test_withdraw_from_hopper_insufficient_funds_fails() {
        let (mut ctx, _stake_pool_accounts, authority, deposit_authority_base, deposit_stake_authority) =
            setup().await;

        // Initialize whitelist
        let mut whitelist_management_program_client = WhitelistManagementProgramClient::new(
            ctx.banks_client.clone(),
            ctx.payer.insecure_clone(),
        );
        let admin = Keypair::new();
        airdrop_lamports(&mut ctx, &admin.pubkey(), LAMPORTS_PER_SOL).await;
        whitelist_management_program_client
            .do_initialize_whitelist(admin.pubkey())
            .await;
        let whitelist_pda = whitelist_management_program_client.get_whitelist_pda();

        // Get hopper PDA and fund it with a small amount
        let mut stake_deposit_interceptor_program_client =
            StakeDepositInterceptorProgramClient::new(
                ctx.banks_client.clone(),
                ctx.payer.insecure_clone(),
            );
        let hopper_pda = stake_deposit_interceptor_program_client.get_hopper_pda(&whitelist_pda);
        airdrop_lamports(&mut ctx, &hopper_pda, LAMPORTS_PER_SOL).await;

        let (deposit_stake_authority_pubkey, _) = derive_stake_pool_deposit_stake_authority(
            &stake_deposit_interceptor_program::id(),
            &deposit_stake_authority.stake_pool,
            &deposit_authority_base.pubkey(),
        );

        let recipient = Keypair::new();
        // Try to withdraw more than the hopper has
        let result = stake_deposit_interceptor_program_client
            .withdraw_from_hopper(
                &authority,
                deposit_stake_authority_pubkey,
                whitelist_pda,
                hopper_pda,
                recipient.pubkey(),
                5 * LAMPORTS_PER_SOL,
            )
            .await;

        // Should fail because hopper doesn't have enough funds
        assert!(result.is_err());
    }
}
