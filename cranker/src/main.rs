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
    dotenv::dotenv,
};

fn load_config() -> Result<CrankerConfig, Box<dyn std::error::Error>> {
        // Load .env file
        dotenv().ok();
    // Load each environment variable with better error messages
    let rpc_url = std::env::var("RPC_URL")
        .map_err(|_| "RPC_URL not found in environment")?;
    
    let ws_url = std::env::var("WS_URL")
        .map_err(|_| "WS_URL not found in environment")?;
    
    let keypair_path = std::env::var("KEYPAIR_PATH")
        .map_err(|_| "KEYPAIR_PATH not found in environment")?;
    
    let payer = Arc::new(read_keypair_file(&keypair_path)
        .map_err(|_| format!("Failed to read keypair from {}", keypair_path))?);
    
    let program_id = Pubkey::from_str(&std::env::var("PROGRAM_ID")
        .map_err(|_| "PROGRAM_ID not found in environment")?)
        .map_err(|_| "Invalid PROGRAM_ID format")?;
    
    let interval = Duration::from_secs(
        std::env::var("INTERVAL_SECONDS")
            .map_err(|_| "INTERVAL_SECONDS not found in environment")?
            .parse()
            .map_err(|_| "INTERVAL_SECONDS must be a valid number")?
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