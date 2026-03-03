use solana_keypair::{Keypair, Signer};
use solana_program::program_pack::Pack;
use solana_program_test::ProgramTestContext;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction::create_account;
use solana_transaction::Transaction;
use spl_token_interface::state::{Account, Mint};

/// Create a SPL Token mint account and return the Pubkey.
/// ProgramTestContext `payer`` is the Mint's `mint_authority`.`
pub async fn create_mint(ctx: &mut ProgramTestContext) -> Pubkey {
    let keypair = Keypair::new();
    let rent = ctx.banks_client.get_rent().await.unwrap();
    let init_account_ix = create_account(
        &ctx.payer.pubkey(),
        &keypair.pubkey(),
        rent.minimum_balance(Mint::LEN),
        Mint::LEN as u64,
        &spl_token_interface::id(),
    );
    let init_mint_ix = spl_token_interface::instruction::initialize_mint(
        &spl_token_interface::id(),
        &keypair.pubkey(),
        &ctx.payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[init_account_ix, init_mint_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &keypair],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    keypair.pubkey()
}

/// Create a SPL Token Account owned by the ProgramTestContext `payer`
pub async fn create_token_account(
    ctx: &mut ProgramTestContext,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    let keypair = Keypair::new();
    let rent = ctx.banks_client.get_rent().await.unwrap();

    let init_account_ix = create_account(
        &ctx.payer.pubkey(),
        &keypair.pubkey(),
        rent.minimum_balance(Account::LEN),
        Account::LEN as u64,
        &spl_token_interface::id(),
    );
    let init_token_account_ix = spl_token_interface::instruction::initialize_account3(
        &spl_token_interface::id(),
        &keypair.pubkey(),
        mint,
        owner,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[init_account_ix, init_token_account_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &keypair],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    keypair.pubkey()
}
