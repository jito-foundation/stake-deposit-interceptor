use ::{
    solana_sdk::{
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
        compute_budget::ComputeBudgetInstruction,
        commitment_config::{ CommitmentConfig, CommitmentLevel },
    },
    solana_client::{
        rpc_client::RpcClient,
        rpc_config::{ RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcSendTransactionConfig },
        rpc_filter::{ Memcmp, RpcFilterType },
        nonce_utils::get_account,
    },
    std::{ sync::Arc, time::{ Duration, SystemTime, UNIX_EPOCH } },
    tokio::time,
    tracing::{ info, error },
    stake_deposit_interceptor::{
        state::{
            DepositReceipt,
            StakePoolDepositStakeAuthority,
            StakeDepositInterceptorDiscriminators,
        },
        instruction::create_claim_pool_tokens_instruction,
    },
    spl_associated_token_account::get_associated_token_address,
    std::thread,
    solana_program::program_pack::Pack,
    spl_token::state::Account as TokenAccount,
};

use spl_associated_token_account::instruction::create_associated_token_account;

#[derive(Clone)]
pub struct CrankerConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub program_id: Pubkey,
    pub payer: Arc<Keypair>, // Wrapped in Arc
    pub interval: Duration,
    pub commitment: CommitmentConfig,
}

pub struct InterceptorCranker {
    rpc_client: Arc<RpcClient>,
    payer: Arc<Keypair>,
    program_id: Pubkey,
    metrics: Arc<std::sync::Mutex<Metrics>>,
    interval: Duration,
}

impl InterceptorCranker {
    pub fn new(config: CrankerConfig) -> Self {
        let rpc_client = Arc::new(
            RpcClient::new_with_commitment(config.rpc_url, config.commitment)
        );

        Self {
            rpc_client,
            payer: config.payer, // No need to clone Arc
            program_id: config.program_id,
            metrics: Arc::new(std::sync::Mutex::new(Metrics::default())),
            interval: config.interval, // Store the interval
        }
    }

    pub async fn start(&self) {
        info!("Starting InterceptorCranker service");
        let mut interval_timer = time::interval(self.interval);
        info!("Set interval timer to {} seconds", self.interval.as_secs());

        loop {
            interval_timer.tick().await;
            info!("Tick: Starting new processing cycle");

            match self.process_expired_receipts().await {
                Ok(_) => info!("Successfully processed expired receipts"),
                Err(e) => error!("Error processing receipts: {}", e),
            }
        }
    }

    async fn process_expired_receipts(&self) -> Result<(), CrankerError> {
        info!("Starting to process expired receipts");
        let receipts = self.get_deposit_receipts().await?;
        info!("Found {} deposit receipts", receipts.len());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| CrankerError::TimeError(e.to_string()))?
            .as_secs();

        for receipt in receipts {
            // Get raw bytes using bytemuck and interpret as little-endian
            let deposit_time_bytes = bytemuck::bytes_of(&receipt.deposit_time);
            let deposit_time = u64::from_le_bytes(deposit_time_bytes.try_into().unwrap());

            let cool_down_bytes = bytemuck::bytes_of(&receipt.cool_down_seconds);
            let cool_down = u64::from_le_bytes(cool_down_bytes.try_into().unwrap());

            info!(
                "Receipt {} raw bytes:\n\
                 deposit_time: {:?}\n\
                 cool_down: {:?}\n\
                 Interpreted values:\n\
                 deposit_time: {}\n\
                 cool_down: {}\n\
                 current_time: {}",
                receipt.base,
                deposit_time_bytes,
                cool_down_bytes,
                deposit_time,
                cool_down,
                now
            );

            if deposit_time > now {
                info!(
                    "Receipt {} not yet expired (future deposit time). Current time: {}, Deposit time: {}",
                    receipt.base,
                    now,
                    deposit_time
                );
                continue;
            }

            // Safe addition check
            match deposit_time.checked_add(cool_down) {
                Some(expiry_time) => {
                    if now > expiry_time {
                        info!(
                            "Receipt {} is expired. Current time: {}, Expiry time: {}",
                            receipt.base,
                            now,
                            expiry_time
                        );
                        match self.claim_pool_tokens(&receipt).await {
                            Ok(_) => {
                                info!("Successfully claimed tokens for receipt {}", receipt.base);
                                let mut metrics = self.metrics.lock().unwrap();
                                metrics.successful_claims += 1;
                            }
                            Err(e) => {
                                error!(
                                    "Failed to claim tokens for receipt {}: {}",
                                    receipt.base,
                                    e
                                );
                                let mut metrics = self.metrics.lock().unwrap();
                                metrics.failed_claims += 1;
                            }
                        }
                    } else {
                        info!(
                            "Receipt {} not yet expired. Current time: {}, Expiry time: {}",
                            receipt.base,
                            now,
                            expiry_time
                        );
                    }
                }
                None => {
                    error!(
                        "Receipt {} has invalid timing values - would overflow. Deposit time: {}, Cool down: {}",
                        receipt.base,
                        deposit_time,
                        cool_down
                    );
                }
            }
        }
        Ok(())
    }

    async fn get_deposit_receipts(&self) -> Result<Vec<DepositReceipt>, CrankerError> {
        let discriminator = StakeDepositInterceptorDiscriminators::DepositReceipt as u8;
        info!("Searching for deposit receipts");

        let filters = vec![
            RpcFilterType::Memcmp(
                Memcmp::new_base58_encoded(
                    0, // offset
                    &[discriminator] // data to match
                )
            )
        ];

        let accounts = self.rpc_client
            .get_program_accounts_with_config(&self.program_id, RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                    commitment: Some(CommitmentConfig::confirmed()),
                    ..Default::default()
                },
                ..Default::default()
            })
            .map_err(CrankerError::RpcError)?;

        info!("Found {} raw accounts", accounts.len());

        Ok(
            accounts
                .into_iter()
                .filter_map(|(pubkey, account)| {
                    // Skip the 8-byte discriminator
                    let data = &account.data[8..];

                    // Ensure we have enough data
                    if data.len() < std::mem::size_of::<DepositReceipt>() {
                        error!(
                            "Account {} data too short: {}, expected: {}",
                            pubkey,
                            data.len(),
                            std::mem::size_of::<DepositReceipt>()
                        );
                        return None;
                    }

                    // Take only the bytes we need for DepositReceipt
                    let receipt_data = &data[..std::mem::size_of::<DepositReceipt>()];

                    match bytemuck::try_from_bytes::<DepositReceipt>(receipt_data) {
                        Ok(receipt) => {
                            info!("Successfully deserialized receipt for {}", pubkey);
                            let mut receipt = *receipt;
                            receipt.base = pubkey;
                            Some(receipt)
                        }
                        Err(e) => {
                            error!(
                                "Failed to deserialize receipt for {}: {}. Data length: {}, Expected: {}",
                                pubkey,
                                e,
                                receipt_data.len(),
                                std::mem::size_of::<DepositReceipt>()
                            );
                            None
                        }
                    }
                })
                .collect()
        )
    }

    async fn claim_pool_tokens(&self, receipt: &DepositReceipt) -> Result<(), CrankerError> {
        info!("Starting detailed claim debug for receipt {}", receipt.base);

        // Ensure this method exists or replace it with the correct one
        let stake_pool_deposit_authority = self.get_stake_pool_deposit_authority(
            &receipt.stake_pool_deposit_stake_authority
        ).await?;

        let owner_ata = get_associated_token_address(
            &receipt.owner,
            &stake_pool_deposit_authority.pool_mint
        );

        // Verify PDAs
        let (expected_authority_pda, authority_bump) = Pubkey::find_program_address(
            &[b"stake_deposit_authority", stake_pool_deposit_authority.stake_pool.as_ref()],
            &self.program_id
        );

        info!(
            "PDA Verification:\n  Expected Authority: {}\n  Actual Authority: {}\n  Bump: {}",
            expected_authority_pda,
            receipt.stake_pool_deposit_stake_authority,
            authority_bump
        );

        // After you fetch the fee wallet
        let fee_wallet = get_account(
            &self.rpc_client,
            &stake_pool_deposit_authority.fee_wallet
        ).map_err(|e| {
            // error!("Failed to fetch account Fee Wallet: {}", e);
            e
        });

        // Before creating the claim instruction
        let fee_wallet_token_account = get_associated_token_address(
            &stake_pool_deposit_authority.fee_wallet,
            &stake_pool_deposit_authority.pool_mint
        );

        // Check if account exists, if not create it
        match self.rpc_client.get_account(&fee_wallet_token_account) {
            Ok(_) => {
                info!("Fee wallet token account exists: {}", fee_wallet_token_account);
            }
            Err(_) => {
                info!("Creating fee wallet token account: {}", fee_wallet_token_account);
                let create_ata_ix = create_associated_token_account(
                    &self.payer.pubkey(),
                    &stake_pool_deposit_authority.fee_wallet,
                    &stake_pool_deposit_authority.pool_mint,
                    &spl_token::id()
                );

                let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
                let create_ata_tx = Transaction::new_signed_with_payer(
                    &[create_ata_ix],
                    Some(&self.payer.pubkey()),
                    &[self.payer.as_ref()],
                    recent_blockhash
                );

                self.rpc_client.send_and_confirm_transaction(&create_ata_tx)?;
                info!("Created fee wallet token account");
            }
        }

        // Now proceed with claim
        info!("Creating claim instruction after verification");       

        let claim_ix = create_claim_pool_tokens_instruction(
            &self.program_id,
            &receipt.base,
            &receipt.owner,
            &stake_pool_deposit_authority.vault,
            &owner_ata,
            &fee_wallet_token_account, // Changed from stake_pool_deposit_authority.fee_wallet
            &receipt.stake_pool_deposit_stake_authority,
            &stake_pool_deposit_authority.pool_mint,
            &spl_token::id(),
            true
        );

        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        let claim_tx = Transaction::new_signed_with_payer(
            &[claim_ix],
            Some(&self.payer.pubkey()),
            &[&*self.payer],
            recent_blockhash
        );

        match self.rpc_client.send_and_confirm_transaction(&claim_tx) {
            Ok(sig) => {
                info!(
                    "Successfully claimed pool tokens for receipt {}. Transaction signature: {}",
                    receipt.base,
                    sig
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to claim pool tokens for receipt {}. Error: {}", receipt.base, e);
                Err(CrankerError::RpcError(e))
            }
        }
    }

    async fn get_stake_pool_deposit_authority(
        &self,
        pubkey: &Pubkey
    ) -> Result<StakePoolDepositStakeAuthority, CrankerError> {
        let account = self.rpc_client.get_account(pubkey).map_err(CrankerError::RpcError)?;

        // Skip 8-byte discriminator
        if account.data.len() < 8 + std::mem::size_of::<StakePoolDepositStakeAuthority>() {
            return Err(
                CrankerError::DeserializeError(
                    format!(
                        "Account data too short: {}, expected: {}",
                        account.data.len(),
                        8 + std::mem::size_of::<StakePoolDepositStakeAuthority>()
                    )
                )
            );
        }

        bytemuck
            ::try_from_bytes::<StakePoolDepositStakeAuthority>(&account.data[8..])
            .map(|auth| *auth)
            .map_err(|e| CrankerError::DeserializeError(e.to_string()))
    }
}

#[derive(Default)]
pub struct Metrics {
    pub processed_receipts: u64,
    pub successful_claims: u64,
    pub failed_claims: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum CrankerError {
    #[error("RPC error: {0}")] RpcError(#[from] solana_client::client_error::ClientError),

    #[error("Program error: {0}")] ProgramError(
        #[from] solana_program::program_error::ProgramError,
    ),

    #[error("Transaction error: {0}")] TransactionError(String),

    #[error("Time error: {0}")] TimeError(String),

    #[error("Deserialize error: {0}")] DeserializeError(String),

    #[error("Timeout error: {0}")] TimeoutError(String),

    #[error("Token error: {0}")] TokenError(String),
}
