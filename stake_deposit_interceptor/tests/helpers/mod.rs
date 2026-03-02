pub mod misc;
pub mod spl;
pub mod stake_client;
pub mod stake_deposit_interceptor;
pub mod stake_deposit_interceptor_client;
pub mod stake_pool;
pub mod system;
pub mod sysvar;
pub mod vote;
pub mod whitelist_management_client;

pub use misc::*;
pub use spl::*;
pub use stake_client::*;
#[allow(unused_imports)]
pub use stake_deposit_interceptor::*;
pub use stake_pool::*;
pub use system::*;
#[allow(unused_imports)]
pub use sysvar::*;
pub use vote::*;

use solana_clock::Clock;
use solana_commitment_config::CommitmentLevel;
use solana_keypair::Signer;
use solana_native_token::sol_str_to_lamports;
use solana_program_error::ProgramError;
use solana_program_test::{BanksClientError, ProgramTest, ProgramTestContext};
use solana_pubkey::Pubkey;
use solana_system_interface::instruction::transfer;
use solana_transaction::{InstructionError, Transaction, TransactionError};
use thiserror::Error;

use crate::helpers::{
    stake_deposit_interceptor_client::StakeDepositInterceptorProgramClient,
    whitelist_management_client::WhitelistManagementProgramClient,
};

pub struct TestBuilder {
    context: ProgramTestContext,
}

impl TestBuilder {
    #[allow(dead_code)]
    pub async fn new() -> Self {
        // $ cargo-build-sbf && SBF_OUT_DIR=$(pwd)/target/sbf-solana-solana/release cargo nextest run
        let mut program_test = ProgramTest::default();
        program_test.add_program(
            "jito_whitelist_management_program",
            jito_whitelist_management_client::programs::JITO_WHITELIST_MANAGEMENT_ID,
            None,
        );

        let context = program_test.start_with_context().await;

        Self { context }
    }

    #[allow(dead_code)]
    pub async fn transfer(&mut self, to: &Pubkey, sol: f64) -> Result<(), BanksClientError> {
        let blockhash = self.context.banks_client.get_latest_blockhash().await?;
        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[transfer(
                        &self.context.payer.pubkey(),
                        to,
                        sol_str_to_lamports(&sol.to_string()).unwrap(),
                    )],
                    Some(&self.context.payer.pubkey()),
                    &[&self.context.payer],
                    blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
    }

    #[allow(dead_code)]
    pub fn stake_program_client(&self) -> StakeProgramClient {
        StakeProgramClient::new(
            self.context.banks_client.clone(),
            self.context.payer.insecure_clone(),
        )
    }

    #[allow(dead_code)]
    pub fn stake_deposit_interceptor_program_client(&self) -> StakeDepositInterceptorProgramClient {
        StakeDepositInterceptorProgramClient::new(
            self.context.banks_client.clone(),
            self.context.payer.insecure_clone(),
        )
    }

    #[allow(dead_code)]
    pub fn whitelist_management_program_client(&self) -> WhitelistManagementProgramClient {
        WhitelistManagementProgramClient::new(
            self.context.banks_client.clone(),
            self.context.payer.insecure_clone(),
        )
    }

    #[allow(dead_code)]
    pub async fn warp_slot_incremental(
        &mut self,
        incremental_slots: u64,
    ) -> Result<(), BanksClientError> {
        let clock: Clock = self.context.banks_client.get_sysvar().await?;
        self.context
            .warp_to_slot(clock.slot.checked_add(incremental_slots).unwrap())
            .map_err(|_| BanksClientError::ClientError("failed to warp slot"))?;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum TestError {
    #[error(transparent)]
    BanksClientError(#[from] BanksClientError),

    #[error(transparent)]
    ProgramError(#[from] ProgramError),
}

impl TestError {
    #[allow(dead_code)]
    pub fn to_transaction_error(&self) -> Option<TransactionError> {
        match self {
            TestError::BanksClientError(e) => match e {
                BanksClientError::TransactionError(e) => Some(e.clone()),
                BanksClientError::SimulationError { err, .. } => Some(err.clone()),
                _ => None,
            },
            TestError::ProgramError(_) => None,
        }
    }
}

#[inline(always)]
#[track_caller]
#[allow(dead_code)]
pub fn assert_ix_error<T>(test_error: Result<T, TestError>, ix_error: InstructionError) {
    assert!(test_error.is_err());
    assert_eq!(
        test_error.err().unwrap().to_transaction_error().unwrap(),
        TransactionError::InstructionError(0, ix_error)
    );
}
