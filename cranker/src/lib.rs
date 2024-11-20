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
        info!("Claiming pool tokens for receipt {}", receipt.base);

        // 1. Get the stake pool deposit authority account
        let stake_pool_deposit_authority = self.get_stake_pool_deposit_authority(
            &receipt.stake_pool_deposit_stake_authority
        ).await?;

        info!(
            "Claiming pool tokens for receipt {} with stake pool deposit authority {}",
            receipt.base,
            receipt.stake_pool_deposit_stake_authority
        );

        // 2. Get or create the owner's ATA
        let owner_ata = get_associated_token_address(
            &receipt.owner,
            &stake_pool_deposit_authority.pool_mint
        );
        info!(
            "Checking ATA {} for owner {} and mint {}",
            owner_ata,
            receipt.owner,
            stake_pool_deposit_authority.pool_mint
        );

        // Check if ATA exists
        match self.rpc_client.get_account(&owner_ata) {
            Ok(_) => {
                info!("ATA already exists");
            }
            Err(e) => {
                info!("ATA doesn't exist, creating new one. Error: {}", e);

                // Check if the mint account exists
                info!("Verifying mint account {}", stake_pool_deposit_authority.pool_mint);
                if
                    let Err(e) = self.rpc_client.get_account(
                        &stake_pool_deposit_authority.pool_mint
                    )
                {
                    error!("Mint account not found: {}", e);
                    return Err(CrankerError::RpcError(e));
                }

                let create_ata_ix =
                    spl_associated_token_account::instruction::create_associated_token_account(
                        &self.payer.pubkey(),
                        &receipt.owner,
                        &stake_pool_deposit_authority.pool_mint,
                        &spl_token::id()
                    );

                info!("Created ATA instruction, getting blockhash");
                let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
                info!("Got blockhash: {}", recent_blockhash);

                // Create transaction with higher compute budget
                let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

                info!("Creating ATA transaction with compute budget");
                let ata_tx = Transaction::new_signed_with_payer(
                    &[compute_budget_ix, create_ata_ix],
                    Some(&self.payer.pubkey()),
                    &[&*self.payer],
                    recent_blockhash
                );

                info!("Sending ATA creation transaction");
                match
                    self.rpc_client.send_transaction_with_config(&ata_tx, RpcSendTransactionConfig {
                        skip_preflight: true,
                        preflight_commitment: Some(CommitmentLevel::Confirmed),
                        encoding: None,
                        max_retries: Some(3),
                        min_context_slot: None,
                    })
                {
                    Ok(sig) => {
                        info!("ATA creation transaction sent with signature: {}", sig);

                        // Set timeout duration
                        let timeout = Duration::from_secs(30);
                        let start = SystemTime::now();

                        info!(
                            "Waiting for ATA creation confirmation with {} second timeout",
                            timeout.as_secs()
                        );

                        while SystemTime::now().duration_since(start).unwrap() < timeout {
                            match self.rpc_client.get_signature_status(&sig)? {
                                Some(Ok(_)) => {
                                    info!("ATA creation confirmed");
                                    // Verify ATA exists
                                    match self.rpc_client.get_account(&owner_ata) {
                                        Ok(_) => {
                                            info!("ATA verified to exist");
                                            break;
                                        }
                                        Err(e) => {
                                            error!("ATA not found after creation: {}", e);
                                            return Err(CrankerError::RpcError(e));
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    error!("ATA creation failed: {:?}", e);
                                    return Err(
                                        CrankerError::TransactionError(
                                            format!("ATA creation failed: {:?}", e)
                                        )
                                    );
                                }
                                None => {
                                    info!(
                                        "Waiting for confirmation... ({} seconds elapsed)",
                                        SystemTime::now().duration_since(start).unwrap().as_secs()
                                    );
                                    thread::sleep(Duration::from_secs(2));
                                    continue;
                                }
                            }
                        }

                        if SystemTime::now().duration_since(start).unwrap() >= timeout {
                            error!("ATA creation timed out after {} seconds", timeout.as_secs());
                            return Err(
                                CrankerError::TimeoutError("ATA creation timed out".to_string())
                            );
                        }
                    }
                    Err(e) => {
                        error!("Failed to send ATA creation transaction: {}", e);
                        return Err(CrankerError::RpcError(e));
                    }
                }
            }
        }

        info!("Creating claim instruction");
        let claim_ix = create_claim_pool_tokens_instruction(
            &self.program_id,
            &receipt.base,
            &receipt.owner,
            &stake_pool_deposit_authority.vault,
            &owner_ata,
            &stake_pool_deposit_authority.fee_wallet,
            &receipt.stake_pool_deposit_stake_authority,
            &stake_pool_deposit_authority.pool_mint,
            &spl_token::id(),
            true
        );

        info!("Getting blockhash for claim transaction");
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;

        info!("Creating and signing claim transaction");
        let claim_tx = Transaction::new_signed_with_payer(
            &[claim_ix],
            Some(&self.payer.pubkey()),
            &[&*self.payer],
            recent_blockhash
        );

        info!("Sending claim transaction");
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

    #[error("Timeout error: {0}")] TimeoutError(String),
}
