mod helpers;

use helpers::program_test_context_with_stake_pool_state;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

#[tokio::test]
async fn test_initialize_fees() {
    let mut ctx = program_test_context_with_stake_pool_state().await;

    let authority = Keypair::new();
    let cool_down_period = 100;
    let initial_fee_rate = 20;
    let init_fee_params_ix =
        stake_deposit_interceptor::instruction::create_init_fee_parameters_instruction(
            &stake_deposit_interceptor::id(),
            cool_down_period,
            initial_fee_rate,
            &authority.pubkey(),
        );

    let tx = Transaction::new_signed_with_payer(
        &[init_fee_params_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &authority],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    // TODO assert FeeParamters account

    assert!(true);
}
