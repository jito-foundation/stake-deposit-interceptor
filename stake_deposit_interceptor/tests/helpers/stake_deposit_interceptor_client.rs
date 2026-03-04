use solana_commitment_config::CommitmentLevel;
use solana_keypair::{Keypair, Signer};
use solana_program::sysvar::SysvarId;
use solana_program_test::BanksClient;
use solana_pubkey::Pubkey;
use solana_transaction::{InstructionError, Transaction, TransactionError};
use stake_deposit_interceptor_client::{
    errors::StakeDepositInterceptorError,
    instructions::{DepositStakeWhitelistedBuilder, WithdrawStakeWhitelistedBuilder},
    programs::STAKE_DEPOSIT_INTERCEPTOR_ID,
};
use stake_deposit_interceptor_program::state::hopper::Hopper;

use crate::helpers::TestError;

pub struct StakeDepositInterceptorProgramClient {
    /// Banks client
    pub banks_client: BanksClient,

    /// Payer keypair
    payer: Keypair,
}

impl StakeDepositInterceptorProgramClient {
    #[allow(dead_code)]
    pub const fn new(banks_client: BanksClient, payer: Keypair) -> Self {
        Self {
            banks_client,
            payer,
        }
    }

    #[allow(dead_code)]
    pub fn get_hopper_pda(&self, whitelist: &Pubkey) -> Pubkey {
        Hopper::find_program_address(&STAKE_DEPOSIT_INTERCEPTOR_ID, whitelist).0
    }

    #[allow(clippy::too_many_arguments, dead_code)]
    pub async fn deposit_stake_whitelisted(
        &mut self,
        whitelisted_signer: &Keypair,
        whitelist: Pubkey,
        stake_pool: Pubkey,
        validator_list: Pubkey,
        stake_deposit_authority: Pubkey,
        withdraw_authority: Pubkey,
        deposit_stake: Pubkey,
        validator_stake: Pubkey,
        reserve_stake: Pubkey,
        pool_tokens_to: Pubkey,
        manager_fee_account: Pubkey,
        referral_fee_account: Pubkey,
        pool_mint: Pubkey,
        spl_stake_pool_program: Pubkey,
    ) -> Result<(), TestError> {
        let blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = DepositStakeWhitelistedBuilder::new()
            .whitelisted_signer(whitelisted_signer.pubkey())
            .whitelist(whitelist)
            .stake_pool(stake_pool)
            .validator_list(validator_list)
            .stake_deposit_authority(stake_deposit_authority)
            .withdraw_authority(withdraw_authority)
            .deposit_stake(deposit_stake)
            .validator_stake(validator_stake)
            .reserve_stake(reserve_stake)
            .pool_tokens_to(pool_tokens_to)
            .manager_fee_account(manager_fee_account)
            .referral_fee_account(referral_fee_account)
            .pool_mint(pool_mint)
            .clock(solana_clock::Clock::id())
            .stake_history(solana_stake_interface::stake_history::StakeHistory::id())
            .stake_program(solana_stake_interface::program::id())
            .spl_stake_pool_program(spl_stake_pool_program)
            .add_remaining_account(solana_program::instruction::AccountMeta::new_readonly(
                solana_system_interface::program::id(),
                false,
            ))
            .instruction();
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, whitelisted_signer],
            blockhash,
        ))
        .await
    }

    #[allow(clippy::too_many_arguments, dead_code)]
    pub async fn withdraw_stake_whitelisted(
        &mut self,
        stake_deposit_authority: Pubkey,
        whitelisted_signer: Keypair,
        whitelist: Pubkey,
        stake_pool: Pubkey,
        validator_list: Pubkey,
        withdraw_authority: Pubkey,
        stake_split_from: Pubkey,
        stake_split_to: Pubkey,
        user_stake_authority: Pubkey,
        user_transfer_authority: Keypair,
        user_pool_token_account: Pubkey,
        manager_fee_account: Pubkey,
        pool_mint: Pubkey,
        fee_rebate_hopper: Pubkey,
        fee_rebate_receiver: Pubkey,
        spl_stake_pool_program_id: Pubkey,
        amount: u64,
    ) -> Result<(), TestError> {
        let blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = WithdrawStakeWhitelistedBuilder::new()
            .stake_deposit_authority(stake_deposit_authority)
            .whitelisted_signer(whitelisted_signer.pubkey())
            .whitelist(whitelist)
            .stake_pool(stake_pool)
            .validator_list(validator_list)
            .withdraw_authority(withdraw_authority)
            .stake_split_from(stake_split_from)
            .stake_split_to(stake_split_to)
            .user_stake_authority(user_stake_authority)
            .user_transfer_authority(user_transfer_authority.pubkey())
            .user_pool_token_account(user_pool_token_account)
            .manager_fee_account(manager_fee_account)
            .pool_mint(pool_mint)
            .fee_rebate_hopper(fee_rebate_hopper)
            .fee_rebate_recipient(fee_rebate_receiver)
            .clock(solana_clock::Clock::id())
            .spl_stake_pool_program(spl_stake_pool_program_id)
            .stake_program(solana_stake_interface::program::id())
            .amount(amount)
            .instruction();
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, &whitelisted_signer, &user_transfer_authority],
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

#[inline(always)]
#[track_caller]
#[allow(dead_code)]
pub fn assert_stake_deposit_interceptor_error<T>(
    test_error: Result<T, TestError>,
    error: StakeDepositInterceptorError,
) {
    assert!(test_error.is_err());
    assert_eq!(
        test_error.err().unwrap().to_transaction_error().unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(error as u32))
    );
}
