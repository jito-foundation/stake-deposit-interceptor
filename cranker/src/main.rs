use {
    stake_deposit_interceptor_cranker::{
        InterceptorCranker,
        CrankerConfig,
    },
    solana_sdk::{
        signature::read_keypair_file,
        pubkey::Pubkey,
        commitment_config::CommitmentConfig,
        signature::Keypair,
    },
    std::{
        str::FromStr,
        time::Duration,
        sync::Arc,  // Add this import
    },
};

fn load_config() -> Result<CrankerConfig, Box<dyn std::error::Error>> {
    let rpc_url = std::env::var("RPC_URL")?;
    let ws_url = std::env::var("WS_URL")?;
    let payer = Arc::new(read_keypair_file(&std::env::var("KEYPAIR_PATH")?)?);  // Wrap in Arc
    let program_id = Pubkey::from_str(&std::env::var("PROGRAM_ID")?)?;
    let interval = Duration::from_secs(
        std::env::var("INTERVAL_SECONDS")?.parse()?
    );

    Ok(CrankerConfig {
        rpc_url,
        ws_url,
        program_id,
        payer,
        interval,
        commitment: CommitmentConfig::confirmed(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = load_config()?;
    
    // Initialize cranker
    let cranker = InterceptorCranker::new(config);

    // Start processing
    cranker.start().await;

    Ok(())
}