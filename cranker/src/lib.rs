use {
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
        transaction::Transaction,
    },
    solana_client::{
        rpc_client::RpcClient,
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        rpc_filter::{Memcmp, RpcFilterType},
    },
    std::{
        sync::Arc,
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
    tokio::time,
    tracing::{info, error},
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
    pub payer: Arc<Keypair>,  // Wrapped in Arc
    pub interval: Duration,
    pub commitment: CommitmentConfig,
}

pub struct InterceptorCranker {
    rpc_client: Arc<RpcClient>,
    payer: Arc<Keypair>,  // Update this too
    program_id: Pubkey,
    metrics: Arc<std::sync::Mutex<Metrics>>,
}

impl InterceptorCranker {
    pub fn new(config: CrankerConfig) -> Self {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            config.rpc_url,
            config.commitment,
        ));

        Self {
            rpc_client,
            payer: config.payer,  // No need to clone Arc
            program_id: config.program_id,
            metrics: Arc::new(std::sync::Mutex::new(Metrics::default())),
        }
    }

    pub async fn start(&self) {
        let interval = Duration::from_secs(60);
        let mut interval_timer = time::interval(interval);

        loop {
            interval_timer.tick().await;
            if let Err(e) = self.process_expired_receipts().await {
                error!("Error processing receipts: {}", e);
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
            if self.is_receipt_expired(&receipt, now) {
                match self.claim_pool_tokens(&receipt).await {
                    Ok(_) => {
                        info!("Successfully claimed tokens for receipt {}", receipt.base);
                        let mut metrics = self.metrics.lock().unwrap();
                        metrics.successful_claims += 1;
                    }
                    Err(e) => {
                        error!("Failed to claim tokens for receipt {}: {}", receipt.base, e);
                        let mut metrics = self.metrics.lock().unwrap();
                        metrics.failed_claims += 1;
                    }
                }
            }
        }
        Ok(())
    }

    async fn get_deposit_receipts(&self) -> Result<Vec<DepositReceipt>, CrankerError> {
        let discriminator = StakeDepositInterceptorDiscriminators::DepositReceipt as u8;
        let filters = vec![
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                0, // offset
                &[discriminator], // data to match
            )),
        ];

        let accounts = self.rpc_client.get_program_accounts_with_config(
            &self.program_id,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                    commitment: Some(CommitmentConfig::confirmed()),
                    ..Default::default()
                },
                ..Default::default()
            },
        ).map_err(CrankerError::RpcError)?;

        Ok(accounts
            .into_iter()
            .filter_map(|(pubkey, account)| {
                bytemuck::try_from_bytes::<DepositReceipt>(&account.data[1..])
                    .ok()
                    .map(|receipt| {
                        let mut receipt = *receipt;
                        receipt.base = pubkey;
                        receipt
                    })
            })
            .collect())
    }

    fn is_receipt_expired(&self, receipt: &DepositReceipt, now: u64) -> bool {
        let deposit_time: u64 = receipt.deposit_time.into();
        let cool_down_seconds: u64 = receipt.cool_down_seconds.into();
        now > (deposit_time + cool_down_seconds)
    }

    async fn claim_pool_tokens(&self, receipt: &DepositReceipt) -> Result<(), CrankerError> {
        // Get the stake pool deposit authority account to access required fields
        let stake_pool_deposit_authority = self.get_stake_pool_deposit_authority(&receipt.stake_pool_deposit_stake_authority).await?;

        let instruction = create_claim_pool_tokens_instruction(
            &self.program_id,
            &receipt.base,
            &receipt.owner,
            &receipt.stake_pool,
            &receipt.stake_pool_deposit_stake_authority,
            &stake_pool_deposit_authority.vault,      // pool_tokens_vault
            &stake_pool_deposit_authority.fee_wallet, // fee_tokens_vault
            &stake_pool_deposit_authority.pool_mint,  // pool_mint
            &stake_pool_deposit_authority.pool_mint,  // validator_stake_account - need to verify this
            false, // is_immutable
        );

        let recent_blockhash = self.rpc_client.get_latest_blockhash()
            .map_err(CrankerError::RpcError)?;
            
            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&self.payer.pubkey()),
                &[&*self.payer],  // Deref the Arc to get &Keypair
                recent_blockhash,
            );

        self.rpc_client.send_and_confirm_transaction(&transaction)
            .map_err(CrankerError::RpcError)?;
            
        Ok(())
    }

    async fn get_stake_pool_deposit_authority(&self, pubkey: &Pubkey) -> Result<StakePoolDepositStakeAuthority, CrankerError> {
        let account = self.rpc_client.get_account(pubkey)
            .map_err(CrankerError::RpcError)?;
        
        bytemuck::try_from_bytes::<StakePoolDepositStakeAuthority>(&account.data[1..])
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
}