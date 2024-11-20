use ::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    },
    solana_client::{
        rpc_client::RpcClient,
        rpc_config::{ RpcAccountInfoConfig, RpcProgramAccountsConfig },
        rpc_filter::{ Memcmp, RpcFilterType },
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
};

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
    payer: Arc<Keypair>, // Update this too
    program_id: Pubkey,
    metrics: Arc<std::sync::Mutex<Metrics>>,
    interval: Duration, // Add this field
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
        info!("Searching for deposit receipts with discriminator: {}", discriminator);
        info!("Expected DepositReceipt size: {}", std::mem::size_of::<DepositReceipt>());

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
        info!(
            "Claiming pool tokens for receipt {} with stake pool deposit authority {}",
            receipt.base,
            receipt.stake_pool_deposit_stake_authority
        );

        // Get the stake pool deposit authority account to access required fields
        let stake_pool_deposit_authority = self.get_stake_pool_deposit_authority(
            &receipt.stake_pool_deposit_stake_authority
        ).await?;

        info!(
            "Creating claim instruction with:\n\
             program_id: {}\n\
             receipt: {}\n\
             owner: {}\n\
             stake_pool: {}\n\
             stake_pool_deposit_stake_authority: {}\n\
             pool_tokens_vault: {}\n\
             fee_tokens_vault: {}\n\
             pool_mint: {}",
            self.program_id,
            receipt.base,
            receipt.owner,
            receipt.stake_pool,
            receipt.stake_pool_deposit_stake_authority,
            stake_pool_deposit_authority.vault,
            stake_pool_deposit_authority.fee_wallet,
            stake_pool_deposit_authority.pool_mint
        );

        let instruction = create_claim_pool_tokens_instruction(
            &self.program_id,
            &receipt.base,
            &receipt.owner,
            &receipt.stake_pool,
            &receipt.stake_pool_deposit_stake_authority,
            &stake_pool_deposit_authority.vault,
            &stake_pool_deposit_authority.fee_wallet,
            &stake_pool_deposit_authority.pool_mint,
            &stake_pool_deposit_authority.pool_mint, // validator_stake_account
            false // is_immutable
        );

        let recent_blockhash = self.rpc_client
            .get_latest_blockhash()
            .map_err(CrankerError::RpcError)?;

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.payer.pubkey()),
            &[&*self.payer],
            recent_blockhash
        );

        self.rpc_client.send_and_confirm_transaction(&transaction).map_err(CrankerError::RpcError)?;

        Ok(())
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

        info!(
            "Deserializing StakePoolDepositStakeAuthority - data length: {}, expected size: {}",
            account.data.len(),
            std::mem::size_of::<StakePoolDepositStakeAuthority>()
        );

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
}
