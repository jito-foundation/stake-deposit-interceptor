use ::{
    jito_bytemuck::AccountDeserialize,
    solana_account_decoder::UiAccountEncoding,
    solana_client::{
        nonblocking::rpc_client::RpcClient,
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig}, // Added explicit imports
        rpc_filter::{Memcmp, RpcFilterType},
    },
    solana_sdk::{
        commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
        transaction::Transaction,
    },
    spl_associated_token_account::{
        get_associated_token_address, instruction::create_associated_token_account,
    },
    stake_deposit_interceptor::{
        instruction::{create_claim_pool_tokens_instruction, derive_stake_deposit_receipt},
        state::{
            DepositReceipt, StakeDepositInterceptorDiscriminators, StakePoolDepositStakeAuthority,
        },
    },
    std::{
        sync::Arc,
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
    tokio::time,
    tracing::{error, info},
};

pub mod metrics;
use metrics::{emit_crank, emit_deposit_receipt, emit_error, emit_heartbeat};

#[derive(Clone)]
pub struct CrankerConfig {
    pub rpc_url: String,
    pub program_id: Pubkey,
    pub payer: Arc<Keypair>, // Wrapped in Arc
    pub interval: Duration,
    pub commitment: CommitmentConfig,
    pub cluster: String,
    pub region: String,
}

pub struct InterceptorCranker {
    rpc_client: Arc<RpcClient>,
    payer: Arc<Keypair>,
    program_id: Pubkey,
    metrics: Arc<std::sync::Mutex<Metrics>>,
    interval: Duration,
    cluster_name: String,
}

impl InterceptorCranker {
    pub fn new(config: CrankerConfig) -> Self {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            config.rpc_url,
            config.commitment,
        ));

        Self {
            rpc_client,
            payer: config.payer, // No need to clone Arc
            program_id: config.program_id,
            metrics: Arc::new(std::sync::Mutex::new(Metrics::default())),
            interval: config.interval, // Store the interval
            cluster_name: config.cluster.clone(),
        }
    }

    pub async fn start(&self) {
        info!("Starting InterceptorCranker service");
        let mut interval_timer = time::interval(self.interval);
        let mut tick: u64 = 0;
        info!("Set interval timer to {} seconds", self.interval.as_secs());

        loop {
            interval_timer.tick().await;
            info!("Tick: Starting new processing cycle");
            match emit_heartbeat(self.rpc_client.clone(), tick, &self.cluster_name).await {
                Ok(_) => tick += 1,
                Err(e) => emit_error(
                    format!("Failed to emit heartbeat: {}", e),
                    &self.cluster_name,
                ),
            }

            match self.process_expired_receipts().await {
                Ok(_) => info!("Successfully processed expired receipts"),
                Err(e) => emit_error(
                    format!("Error processing receipts: {}", e),
                    &self.cluster_name,
                ),
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

        let deposit_receipts: u64 = receipts.len() as u64;
        let mut future_deposits: u64 = 0;
        let mut not_yet_expired_receipts: u64 = 0;
        let mut claimed_receipts: u64 = 0;
        for receipt in receipts {
            // Get raw bytes using bytemuck and interpret as little-endian
            let deposit_time = u64::from(receipt.deposit_time);
            let cool_down = u64::from(receipt.cool_down_seconds);

            info!(
                "Receipt {} raw bytes:\n\
                 Interpreted values:\n\
                 deposit_time: {}\n\
                 cool_down: {}\n\
                 current_time: {}",
                receipt.base, deposit_time, cool_down, now
            );
            emit_deposit_receipt(&receipt, &self.cluster_name);

            if deposit_time > now {
                info!(
                    "Receipt {} not yet expired (future deposit time). Current time: {}, Deposit time: {}",
                    receipt.base,
                    now,
                    deposit_time
                );
                future_deposits += 1;
                continue;
            }

            // Safe addition check
            match deposit_time.checked_add(cool_down) {
                Some(expiry_time) => {
                    if now > expiry_time {
                        info!(
                            "Receipt {} is expired. Current time: {}, Expiry time: {}",
                            receipt.base, now, expiry_time
                        );
                        match self.claim_pool_tokens(&receipt).await {
                            Ok(_) => {
                                info!("Successfully claimed tokens for receipt {}", receipt.base);
                                let mut metrics = self.metrics.lock().unwrap();
                                metrics.successful_claims += 1;
                                claimed_receipts += 1;
                            }
                            Err(e) => {
                                emit_error(
                                    format!(
                                        "Failed to claim tokens for receipt {}: {}",
                                        receipt.base, e
                                    ),
                                    &self.cluster_name,
                                );
                                let mut metrics = self.metrics.lock().unwrap();
                                metrics.failed_claims += 1;
                            }
                        }
                    } else {
                        info!(
                            "Receipt {} not yet expired. Current time: {}, Expiry time: {}",
                            receipt.base, now, expiry_time
                        );
                        not_yet_expired_receipts += 1;
                    }
                }
                None => {
                    emit_error(format!(
                        "Receipt {} has invalid timing values - would overflow. Deposit time: {}, Cool down: {}",
                        receipt.base,
                        deposit_time,
                        cool_down
                    ), &self.cluster_name);
                }
            }
        }

        emit_crank(
            deposit_receipts,
            future_deposits,
            not_yet_expired_receipts,
            claimed_receipts,
            &self.cluster_name,
        );

        Ok(())
    }

    async fn get_deposit_receipts(&self) -> Result<Vec<DepositReceipt>, CrankerError> {
        let discriminator = StakeDepositInterceptorDiscriminators::DepositReceipt as u8;
        info!("Searching for deposit receipts");

        let accounts = self
            .rpc_client
            .get_program_accounts_with_config(
                &self.program_id,
                RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                        0,
                        &[discriminator],
                    ))]),
                    account_config: RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::Base64),
                        commitment: Some(CommitmentConfig::confirmed()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .map_err(CrankerError::RpcError)?;

        info!("Found {} raw accounts", accounts.len());

        Ok(accounts
            .into_iter()
            .filter_map(|(pubkey, account)| {
                match DepositReceipt::try_from_slice_unchecked(account.data.as_slice()) {
                    Ok(receipt) => {
                        info!(
                            "Found receipt:\n\
                                 Account pubkey: {}\n\
                                 Receipt base: {}\n\
                                 Receipt stake pool: {}\n\
                                 Derived PDA: {}",
                            pubkey,
                            receipt.base,
                            receipt.stake_pool,
                            derive_stake_deposit_receipt(
                                &receipt.stake_pool,
                                &pubkey,
                                &self.program_id
                            )
                            .0
                        );

                        Some(*receipt)
                    }
                    Err(e) => {
                        emit_error(
                            format!("Failed to deserialize receipt for {}: {}", pubkey, e),
                            &self.cluster_name,
                        );
                        None
                    }
                }
            })
            .collect())
    }

    async fn claim_pool_tokens(&self, receipt: &DepositReceipt) -> Result<(), CrankerError> {
        info!("Starting detailed claim debug for receipt {}", receipt.base);

        let stake_pool_deposit_authority = self
            .get_stake_pool_deposit_authority(&receipt.stake_pool_deposit_stake_authority)
            .await?;

        let owner_ata =
            get_associated_token_address(&receipt.owner, &stake_pool_deposit_authority.pool_mint);

        let fee_wallet_token_account = get_associated_token_address(
            &stake_pool_deposit_authority.fee_wallet,
            &stake_pool_deposit_authority.pool_mint,
        );

        // Check if account exists
        match self.rpc_client.get_account(&fee_wallet_token_account).await {
            Ok(_) => {
                info!(
                    "Fee wallet token account exists: {}",
                    fee_wallet_token_account
                );
            }
            Err(_) => {
                info!(
                    "Creating fee wallet token account: {}",
                    fee_wallet_token_account
                );
                let create_ata_ix = create_associated_token_account(
                    &self.payer.pubkey(),
                    &stake_pool_deposit_authority.fee_wallet,
                    &stake_pool_deposit_authority.pool_mint,
                    &spl_token::id(),
                );

                let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
                let create_ata_tx = Transaction::new_signed_with_payer(
                    &[create_ata_ix],
                    Some(&self.payer.pubkey()),
                    &[self.payer.as_ref()],
                    recent_blockhash,
                );

                self.rpc_client
                    .send_and_confirm_transaction(&create_ata_tx)
                    .await?;
                info!("Created fee wallet token account");
            }
        }

        // Derive the correct PDA
        let (derived_receipt_address, _) =
            derive_stake_deposit_receipt(&self.program_id, &receipt.stake_pool, &receipt.base);

        let claim_ix = create_claim_pool_tokens_instruction(
            &self.program_id,
            &derived_receipt_address, // Use the derived address here instead of receipt.base
            &receipt.owner,
            &stake_pool_deposit_authority.vault,
            &owner_ata,
            &fee_wallet_token_account,
            &receipt.stake_pool_deposit_stake_authority,
            &stake_pool_deposit_authority.pool_mint,
            &spl_token::id(),
            true,
        );

        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let claim_tx = Transaction::new_signed_with_payer(
            &[claim_ix],
            Some(&self.payer.pubkey()),
            &[&*self.payer],
            recent_blockhash,
        );

        match self
            .rpc_client
            .send_and_confirm_transaction(&claim_tx)
            .await
        {
            Ok(sig) => {
                info!(
                    "Successfully claimed pool tokens for receipt {}. Transaction signature: {}",
                    receipt.base, sig
                );
                Ok(())
            }
            Err(e) => {
                emit_error(
                    format!(
                        "Failed to claim pool tokens for receipt {}. Error: {}",
                        receipt.base, e
                    ),
                    &self.cluster_name,
                );
                Err(CrankerError::RpcError(e))
            }
        }
    }

    async fn get_stake_pool_deposit_authority(
        &self,
        pubkey: &Pubkey,
    ) -> Result<StakePoolDepositStakeAuthority, CrankerError> {
        let account = self
            .rpc_client
            .get_account(pubkey)
            .await
            .map_err(CrankerError::RpcError)?;

        StakePoolDepositStakeAuthority::try_from_slice_unchecked(account.data.as_slice())
            .copied()
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
    #[error("RPC error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),

    #[error("Program error: {0}")]
    ProgramError(#[from] solana_program::program_error::ProgramError),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Time error: {0}")]
    TimeError(String),

    #[error("Deserialize error: {0}")]
    DeserializeError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Token error: {0}")]
    TokenError(String),
}
