mod helpers;

use helpers::{
    assert_transaction_err, clone_account_to_new_address, create_stake_deposit_authority,
    program_test_context_with_stake_pool_state, StakePoolAccounts,
};
use jito_bytemuck::AccountDeserialize;
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    borsh1::try_from_slice_unchecked,
    instruction::{AccountMeta, Instruction, InstructionError},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use stake_deposit_interceptor::{
    error::StakeDepositInterceptorError,
    instruction::{
        derive_stake_pool_deposit_stake_authority, StakeDepositInterceptorInstruction,
        UpdateStakePoolDepositStakeAuthorityArgs,
    },
    state::StakePoolDepositStakeAuthority,
};

#[tokio::test]
async fn test_update_deposit_stake_authority() {
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

    let fee_wallet = Keypair::new();
    let new_authority = Keypair::new();
    let cool_down_seconds = 78;
    let initial_fee_bps = 20;

    let update_ix =
        stake_deposit_interceptor::instruction::create_update_deposit_stake_authority_instruction(
            &stake_deposit_interceptor::id(),
            &stake_pool_accounts.stake_pool,
            &authority.pubkey(),
            &deposit_authority_base.pubkey(),
            Some(new_authority.pubkey()),
            Some(fee_wallet.pubkey()),
            Some(cool_down_seconds),
            Some(initial_fee_bps),
        );

    let tx = Transaction::new_signed_with_payer(
        &[update_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &authority],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    let (deposit_stake_authority_pubkey, _bump_seed) = derive_stake_pool_deposit_stake_authority(
        &stake_deposit_interceptor::ID,
        &stake_pool_accounts.stake_pool,
        &deposit_authority_base.pubkey(),
    );

    let account = ctx
        .banks_client
        .get_account(deposit_stake_authority_pubkey)
        .await
        .unwrap()
        .unwrap();

    let deposit_stake_authority =
        StakePoolDepositStakeAuthority::try_from_slice_unchecked(&account.data.as_slice()).unwrap();

    let actual_cool_down_seconds: u64 = deposit_stake_authority.cool_down_seconds.into();
    let actual_initial_fee_bps: u32 = deposit_stake_authority.inital_fee_bps.into();
    assert_eq!(actual_cool_down_seconds, cool_down_seconds);
    assert_eq!(actual_initial_fee_bps, initial_fee_bps);
    assert_eq!(deposit_stake_authority.fee_wallet, fee_wallet.pubkey());
    assert_eq!(deposit_stake_authority.authority, new_authority.pubkey());
}

async fn setup_with_ix() -> (
    ProgramTestContext,
    StakePoolAccounts,
    Keypair,
    Keypair,
    Pubkey,
    Instruction,
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

    let fee_wallet = Keypair::new();
    let new_authority = Keypair::new();
    let cool_down_seconds = 78;
    let initial_fee_bps = 20;

    let update_ix =
        stake_deposit_interceptor::instruction::create_update_deposit_stake_authority_instruction(
            &stake_deposit_interceptor::id(),
            &stake_pool_accounts.stake_pool,
            &authority.pubkey(),
            &deposit_authority_base.pubkey(),
            Some(new_authority.pubkey()),
            Some(fee_wallet.pubkey()),
            Some(cool_down_seconds),
            Some(initial_fee_bps),
        );

    let (deposit_stake_authority_pubkey, _bump) = derive_stake_pool_deposit_stake_authority(
        &stake_deposit_interceptor::id(),
        &stake_pool_accounts.stake_pool,
        &deposit_authority_base.pubkey(),
    );
    (
        ctx,
        stake_pool_accounts,
        authority,
        new_authority,
        deposit_stake_authority_pubkey,
        update_ix,
    )
}

#[tokio::test]
async fn test_fail_program_does_not_own_pda_account() {
    let (
        mut ctx,
        _stake_pool_accounts,
        authority,
        _new_authority,
        _deposit_stake_authority_pubkey,
        mut ix,
    ) = setup_with_ix().await;
    ix.accounts[0] = AccountMeta::new(Pubkey::new_unique(), false);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &authority],
        ctx.last_blockhash,
    );

    assert_transaction_err(&mut ctx, tx, InstructionError::IncorrectProgramId).await;
}

#[tokio::test]
async fn test_fail_authority_not_signer() {
    let (
        mut ctx,
        _stake_pool_accounts,
        authority,
        _new_authority,
        _deposit_stake_authority_pubkey,
        mut ix,
    ) = setup_with_ix().await;
    ix.accounts[1] = AccountMeta::new_readonly(authority.pubkey(), false);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(StakeDepositInterceptorError::SignatureMissing as u32),
    )
    .await;
}

#[tokio::test]
async fn test_fail_authority_incorrect() {
    let (
        mut ctx,
        _stake_pool_accounts,
        _authority,
        _new_authority,
        _deposit_stake_authority_pubkey,
        mut ix,
    ) = setup_with_ix().await;
    let bad_authority = Keypair::new();
    ix.accounts[1] = AccountMeta::new_readonly(bad_authority.pubkey(), true);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &bad_authority],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(StakeDepositInterceptorError::InvalidAuthority as u32),
    )
    .await;
}

#[tokio::test]
async fn test_fail_invalid_stake_deposit_authority_address() {
    let (
        mut ctx,
        _stake_pool_accounts,
        authority,
        _new_authority,
        deposit_stake_authority_pubkey,
        mut ix,
    ) = setup_with_ix().await;
    let bad_account = clone_account_to_new_address(&mut ctx, &deposit_stake_authority_pubkey).await;
    ix.accounts[0] = AccountMeta::new(bad_account, false);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &authority],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(
            StakeDepositInterceptorError::InvalidStakePoolDepositStakeAuthority as u32,
        ),
    )
    .await;
}

#[tokio::test]
async fn test_fail_initial_fee_bps_cannot_exceed_10000() {
    let (
        mut ctx,
        _stake_pool_accounts,
        authority,
        _new_authority,
        _deposit_stake_authority_pubkey,
        mut ix,
    ) = setup_with_ix().await;

    let args = UpdateStakePoolDepositStakeAuthorityArgs {
        fee_wallet: None,
        initial_fee_bps: Some(10_001),
        cool_down_seconds: None,
    };
    ix.data = borsh::to_vec(
        &StakeDepositInterceptorInstruction::UpdateStakePoolDepositStakeAuthority(args),
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &authority],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(StakeDepositInterceptorError::InitialFeeRateMaxExceeded as u32),
    )
    .await;
}
