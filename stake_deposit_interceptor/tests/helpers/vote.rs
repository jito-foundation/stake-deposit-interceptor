use solana_hash::Hash;
use solana_keypair::{Keypair, Signer};
use solana_program_test::BanksClient;
use solana_system_interface::instruction::create_account;
use solana_transaction::Transaction;
use solana_vote_interface::state::VoteInit;
use spl_pod::solana_program::vote::state::VoteState;

pub async fn create_vote(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    validator: &Keypair,
    vote: &Keypair,
) {
    let rent = banks_client.get_rent().await.unwrap();
    let rent_voter = rent.minimum_balance(VoteState::size_of());

    let mut instructions = vec![create_account(
        &payer.pubkey(),
        &validator.pubkey(),
        rent.minimum_balance(0),
        0,
        &solana_system_interface::program::id(),
    )];
    instructions.append(
        &mut solana_vote_interface::instruction::create_account_with_config(
            &payer.pubkey(),
            &vote.pubkey(),
            &VoteInit {
                node_pubkey: validator.pubkey(),
                authorized_voter: validator.pubkey(),
                ..VoteInit::default()
            },
            rent_voter,
            solana_vote_interface::instruction::CreateVoteAccountConfig {
                space: VoteState::size_of() as u64,
                ..Default::default()
            },
        ),
    );

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[validator, vote, payer],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();
}
