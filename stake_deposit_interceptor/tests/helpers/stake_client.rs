use solana_commitment_config::CommitmentLevel;
use solana_keypair::{Keypair, Signer};
use solana_program_test::BanksClient;
use solana_pubkey::Pubkey;
use solana_stake_interface::state::StakeAuthorize;
use solana_transaction::Transaction;

use crate::helpers::TestError;

#[allow(dead_code)]
pub struct StakeProgramClient {
    /// Banks client
    pub banks_client: BanksClient,

    /// Payer keypair
    payer: Keypair,
}

impl StakeProgramClient {
    #[allow(dead_code)]
    pub const fn new(banks_client: BanksClient, payer: Keypair) -> Self {
        Self {
            banks_client,
            payer,
        }
    }

    #[allow(dead_code)]
    pub async fn authorize(
        &mut self,
        stake_pubkey: &Pubkey,
        authorized: &Keypair,
        new_authorized: &Pubkey,
        stake_authorize: StakeAuthorize,
    ) -> Result<(), TestError> {
        let authorize_ix = solana_stake_interface::instruction::authorize(
            stake_pubkey,
            &authorized.pubkey(),
            new_authorized,
            stake_authorize,
            None,
        );

        let blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[authorize_ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, authorized],
            blockhash,
        ))
        .await
    }

    pub async fn process_transaction(&mut self, tx: &Transaction) -> Result<(), TestError> {
        self.banks_client
            .process_transaction_with_preflight_and_commitment(
                tx.clone(),
                CommitmentLevel::Processed,
            )
            .await?;

        Ok(())
    }
}
