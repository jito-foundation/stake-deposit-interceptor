mod helpers;

use helpers::{
    assert_transaction_err, program_test_context_with_stake_pool_state, StakePoolAccounts,
};
use jito_bytemuck::AccountDeserialize;
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction, InstructionError},
    program_pack::Pack,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction::transfer,
    system_program,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use stake_deposit_interceptor::{
    error::StakeDepositInterceptorError, instruction::derive_stake_pool_deposit_stake_authority,
    state::StakePoolDepositStakeAuthority,
};

#[tokio::test]
async fn test_init_deposit_stake_authority() {
    let (mut ctx, stake_pool_accounts) = program_test_context_with_stake_pool_state().await;

    let deposit_authority_base = Keypair::new();
    let fee_wallet = Keypair::new();
    let authority = Keypair::new();
    let cool_down_seconds = 100;
    let initial_fee_bps = 20;
    let init_ix =
        stake_deposit_interceptor::instruction::create_init_deposit_stake_authority_instruction(
            &stake_deposit_interceptor::id(),
            &ctx.payer.pubkey(),
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.pool_mint,
            &spl_stake_pool::id(),
            &spl_token::id(),
            &fee_wallet.pubkey(),
            cool_down_seconds,
            initial_fee_bps,
            &authority.pubkey(),
            &deposit_authority_base.pubkey(),
        );

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    let (deposit_stake_authority_pubkey, _bump_seed) = derive_stake_pool_deposit_stake_authority(
        &stake_deposit_interceptor::ID,
        &stake_pool_accounts.stake_pool,
        &deposit_authority_base.pubkey(),
    );
    let vault_ata = get_associated_token_address(
        &deposit_stake_authority_pubkey,
        &stake_pool_accounts.pool_mint,
    );

    let account = ctx
        .banks_client
        .get_account(deposit_stake_authority_pubkey)
        .await
        .unwrap()
        .unwrap();

    let vault_account = ctx
        .banks_client
        .get_account(vault_ata)
        .await
        .unwrap()
        .unwrap();

    let deposit_stake_authority =
        StakePoolDepositStakeAuthority::try_from_slice_unchecked(&account.data.as_slice()).unwrap();
    let vault_token_account =
        spl_token::state::Account::unpack(vault_account.data.as_slice()).unwrap();
    assert_eq!(vault_token_account.mint, stake_pool_accounts.pool_mint);
    assert_eq!(vault_token_account.amount, 0);
    assert_eq!(vault_token_account.owner, deposit_stake_authority_pubkey);

    assert_eq!(deposit_stake_authority.authority, authority.pubkey());
    let actual_cool_down_seconds: u64 = deposit_stake_authority.cool_down_seconds.into();
    let actual_initial_fee_bps: u32 = deposit_stake_authority.inital_fee_bps.into();
    assert_eq!(actual_cool_down_seconds, cool_down_seconds);
    assert_eq!(actual_initial_fee_bps, initial_fee_bps);
    assert_eq!(
        deposit_stake_authority.base,
        deposit_authority_base.pubkey()
    );
    assert_eq!(
        deposit_stake_authority.stake_pool,
        stake_pool_accounts.stake_pool
    );
    assert_eq!(
        deposit_stake_authority.pool_mint,
        stake_pool_accounts.pool_mint
    );
    assert_eq!(
        deposit_stake_authority.stake_pool_program_id,
        spl_stake_pool::id()
    );
    assert_eq!(deposit_stake_authority.fee_wallet, fee_wallet.pubkey());
    assert_eq!(deposit_stake_authority.vault, vault_ata);
}

async fn setup_with_ix() -> (
    ProgramTestContext,
    StakePoolAccounts,
    Keypair,
    Keypair,
    Instruction,
) {
    let (ctx, stake_pool_accounts) = program_test_context_with_stake_pool_state().await;

    let deposit_authority_base = Keypair::new();
    let fee_wallet = Keypair::new();
    let authority = Keypair::new();
    let cool_down_seconds = 100;
    let initial_fee_bps = 20;
    let ix =
        stake_deposit_interceptor::instruction::create_init_deposit_stake_authority_instruction(
            &stake_deposit_interceptor::id(),
            &ctx.payer.pubkey(),
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.pool_mint,
            &spl_stake_pool::id(),
            &spl_token::id(),
            &fee_wallet.pubkey(),
            cool_down_seconds,
            initial_fee_bps,
            &authority.pubkey(),
            &deposit_authority_base.pubkey(),
        );
    (
        ctx,
        stake_pool_accounts,
        authority,
        deposit_authority_base,
        ix,
    )
}

#[tokio::test]
async fn test_fail_invalid_system_program() {
    let (mut ctx, _stake_pool_accounts, _authority, deposit_authority_base, mut init_ix) =
        setup_with_ix().await;
    init_ix.accounts[10] = AccountMeta::new_readonly(Pubkey::new_unique(), false);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    assert_transaction_err(&mut ctx, tx, InstructionError::IncorrectProgramId).await;
}

#[tokio::test]
async fn test_success_with_prefunded_account() {
    let (mut ctx, stake_pool_accounts, _authority, deposit_authority_base, init_ix) =
        setup_with_ix().await;
    let (deposit_stake_authority_pubkey, _bump_seed) = derive_stake_pool_deposit_stake_authority(
        &stake_deposit_interceptor::id(),
        &stake_pool_accounts.stake_pool,
        &deposit_authority_base.pubkey(),
    );

    let transfer_ix = transfer(&ctx.payer.pubkey(), &deposit_stake_authority_pubkey, 100);

    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix, init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();
}

#[tokio::test]
async fn test_fail_invalid_deposit_stake_authority_owner() {
    let (mut ctx, stake_pool_accounts, _authority, deposit_authority_base, init_ix) =
        setup_with_ix().await;
    let (deposit_stake_authority_pubkey, _bump_seed) = derive_stake_pool_deposit_stake_authority(
        &stake_deposit_interceptor::id(),
        &stake_pool_accounts.stake_pool,
        &deposit_authority_base.pubkey(),
    );
    let bad_account = AccountSharedData::new(1, 0, &stake_deposit_interceptor::id());
    ctx.set_account(&deposit_stake_authority_pubkey, &bad_account);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    assert_transaction_err(&mut ctx, tx.clone(), InstructionError::InvalidAccountOwner).await;
}

#[tokio::test]
async fn test_fail_deposit_stake_authority_not_empty() {
    let (mut ctx, stake_pool_accounts, _authority, deposit_authority_base, init_ix) =
        setup_with_ix().await;
    let (deposit_stake_authority_pubkey, _bump_seed) = derive_stake_pool_deposit_stake_authority(
        &stake_deposit_interceptor::id(),
        &stake_pool_accounts.stake_pool,
        &deposit_authority_base.pubkey(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    let bad_account = AccountSharedData::new(1, 1, &system_program::id());
    ctx.set_account(&deposit_stake_authority_pubkey, &bad_account);
    assert_transaction_err(
        &mut ctx,
        tx.clone(),
        InstructionError::AccountAlreadyInitialized,
    )
    .await;
}

#[tokio::test]
async fn test_fail_base_non_signer() {
    let (mut ctx, _stake_pool_accounts, _authority, deposit_authority_base, mut init_ix) =
        setup_with_ix().await;
    init_ix.accounts[4] = AccountMeta::new_readonly(deposit_authority_base.pubkey(), false);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
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
async fn test_fail_incorrect_stakepool_program() {
    let (mut ctx, _stake_pool_accounts, _authority, deposit_authority_base, mut init_ix) =
        setup_with_ix().await;
    init_ix.accounts[7] = AccountMeta::new_readonly(Pubkey::new_unique(), false);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(StakeDepositInterceptorError::InvalidStakePool as u32),
    )
    .await;
}

#[tokio::test]
async fn test_fail_incorrect_stakepool_mint() {
    let (mut ctx, _stake_pool_accounts, _authority, deposit_authority_base, mut init_ix) =
        setup_with_ix().await;
    init_ix.accounts[6] = AccountMeta::new_readonly(Pubkey::new_unique(), false);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(StakeDepositInterceptorError::InvalidStakePool as u32),
    )
    .await;
}

#[tokio::test]
async fn test_fail_incorrect_token_program() {
    let (mut ctx, _stake_pool_accounts, _authority, deposit_authority_base, mut init_ix) =
        setup_with_ix().await;
    init_ix.accounts[8] = AccountMeta::new_readonly(spl_token_2022::id(), false);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(StakeDepositInterceptorError::InvalidTokenProgram as u32),
    )
    .await;
}

#[tokio::test]
async fn test_fail_incorrect_deposit_stake_authority() {
    let (mut ctx, _stake_pool_accounts, _authority, deposit_authority_base, mut init_ix) =
        setup_with_ix().await;
    init_ix.accounts[1] = AccountMeta::new(Pubkey::new_unique(), false);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(StakeDepositInterceptorError::InvalidSeeds as u32),
    )
    .await;
}

#[tokio::test]
async fn test_fail_incorrect_vault() {
    let (mut ctx, _stake_pool_accounts, _authority, deposit_authority_base, mut init_ix) =
        setup_with_ix().await;
    init_ix.accounts[2] = AccountMeta::new(Pubkey::new_unique(), false);

    let tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(StakeDepositInterceptorError::InvalidVault as u32),
    )
    .await;
}

#[tokio::test]
async fn test_fail_initial_fee_bps_cannot_exceed_10000() {
    let (mut ctx, stake_pool_accounts) = program_test_context_with_stake_pool_state().await;

    let deposit_authority_base = Keypair::new();
    let fee_wallet = Keypair::new();
    let authority = Keypair::new();
    let cool_down_seconds = 100;
    let initial_fee_bps = 10_001;
    let ix =
        stake_deposit_interceptor::instruction::create_init_deposit_stake_authority_instruction(
            &stake_deposit_interceptor::id(),
            &ctx.payer.pubkey(),
            &stake_pool_accounts.stake_pool,
            &stake_pool_accounts.pool_mint,
            &spl_stake_pool::id(),
            &spl_token::id(),
            &fee_wallet.pubkey(),
            cool_down_seconds,
            initial_fee_bps,
            &authority.pubkey(),
            &deposit_authority_base.pubkey(),
        );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &deposit_authority_base],
        ctx.last_blockhash,
    );

    assert_transaction_err(
        &mut ctx,
        tx,
        InstructionError::Custom(StakeDepositInterceptorError::InitialFeeRateMaxExceeded as u32),
    )
    .await;
}
