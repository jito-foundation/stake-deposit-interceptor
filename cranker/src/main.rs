use ::{
    stake_deposit_interceptor_cranker::{ InterceptorCranker, CrankerConfig },
    solana_metrics::set_host_id,
    solana_sdk::{
        signature::{ read_keypair_file, Signer }, // Added Signer trait
        pubkey::Pubkey,
        commitment_config::CommitmentConfig,
    },
    std::{ str::FromStr, time::Duration, process::Command, sync::Arc },
    dotenv::dotenv,
    tracing::{ info, Level },
};

fn load_config() -> Result<CrankerConfig, Box<dyn std::error::Error>> {
    // Load .env file
    dotenv().ok();
    // Load each environment variable with better error messages
    let rpc_url = std::env::var("RPC_URL").map_err(|_| "RPC_URL not found in environment")?;

    let ws_url = std::env::var("WS_URL").map_err(|_| "WS_URL not found in environment")?;

    let keypair_path = std::env
        ::var("KEYPAIR_PATH")
        .map_err(|_| "KEYPAIR_PATH not found in environment")?;

    let payer = Arc::new(
        read_keypair_file(&keypair_path).map_err(|_|
            format!("Failed to read keypair from {}", keypair_path)
        )?
    );

    let program_id = Pubkey::from_str(
        &std::env::var("PROGRAM_ID").map_err(|_| "PROGRAM_ID not found in environment")?
    ).map_err(|_| "Invalid PROGRAM_ID format")?;

    let interval = Duration::from_secs(
        std::env
            ::var("INTERVAL_SECONDS")
            .map_err(|_| "INTERVAL_SECONDS not found in environment")?
            .parse()
            .map_err(|_| "INTERVAL_SECONDS must be a valid number")?
    );

    let cluster = std::env::var("CLUSTER")
        .map_err(|_| "CLUSTER not found in environment")?;

    let region = std::env::var("REGION")
        .map_err(|_| "REGION not found in environment")?;
 
    Ok(CrankerConfig {
        rpc_url,
        ws_url,
        program_id,
        payer,
        interval,
        commitment: CommitmentConfig::confirmed(),
        cluster: cluster,
        region: region,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with a simpler configuration
    tracing_subscriber
        ::fmt() // Use fully qualified path
        .with_max_level(Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .pretty()
        .init();

    info!("Logger initialized");

    // Load .env file
    dotenv().ok();
    info!("Environment loaded");

    // Load configuration
    let config = load_config()?;

    info!("Configuration loaded successfully:");
    info!("RPC URL: {}", config.rpc_url);
    info!("WS URL: {}", config.ws_url);
    info!("Program ID: {}", config.program_id);
    info!("Payer: {}", config.payer.as_ref().pubkey()); // Signer trait now in scope
    info!("Interval: {}s", config.interval.as_secs());
    info!("Cluster: {}", config.cluster);
    info!("Region: {}", config.region);

    // Set host ID
    let hostname_cmd = Command::new("hostname")
        .output()
        .expect("Failed to execute hostname command");

    let hostname = String::from_utf8_lossy(&hostname_cmd.stdout)
        .trim()
        .to_string();

    set_host_id(format!(
        "interceptor-cranker_{}_{}_{}",
        config.region, config.cluster, hostname
    ));

    // Initialize cranker
    let cranker = InterceptorCranker::new(config);
    info!("Cranker initialized");

    // Start processing
    info!("Starting cranker service...");
    cranker.start().await;

    Ok(())
}
