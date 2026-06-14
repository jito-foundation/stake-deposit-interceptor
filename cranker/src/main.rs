use std::{process::Command, sync::Arc, time::Duration};

use clap::Parser;
use ::{
    dotenv::dotenv,
    solana_commitment_config::CommitmentConfig,
    solana_metrics::set_host_id,
    solana_sdk::{
        pubkey::Pubkey,
        signature::{read_keypair_file, Signer}, // Added Signer trait
    },
    stake_deposit_interceptor_cranker::{CrankerConfig, InterceptorCranker},
    tracing::{info, Level},
};

#[derive(Parser)]
#[command(author, version, about = "Cranker for the stake deposit interceptor", long_about = None)]
pub struct Cli {
    /// RPC URL to use
    #[arg(long, env)]
    pub rpc_url: String,

    /// Filepath to the payer keypair
    #[arg(long, env)]
    pub keypair_path: String,

    /// Interceptor program ID
    #[arg(long, env)]
    pub program_id: Pubkey,

    /// Interval between cranker iterations in seconds
    #[arg(long, env, default_value = "3600")]
    pub interval_seconds: u64,

    /// Cluster name for metrics (e.g., "mainnet", "testnet")
    #[arg(long, env)]
    pub cluster: String,

    /// Region name for metrics
    #[arg(long, env)]
    pub region: String,

    /// Stake pool deposit stake authority pubkey
    #[arg(long, env)]
    pub stake_pool_deposit_stake_authority: Pubkey,

    /// Whitelist pubkey
    #[arg(long, env)]
    pub whitelist: Pubkey,
}

impl TryFrom<Cli> for CrankerConfig {
    type Error = Box<dyn std::error::Error>;

    fn try_from(cli: Cli) -> Result<Self, Self::Error> {
        let payer = Arc::new(
            read_keypair_file(&cli.keypair_path)
                .map_err(|_| format!("Failed to read keypair from {}", cli.keypair_path))?,
        );

        Ok(CrankerConfig {
            rpc_url: cli.rpc_url,
            program_id: cli.program_id,
            payer,
            interval: Duration::from_secs(cli.interval_seconds),
            commitment: CommitmentConfig::confirmed(),
            cluster: cli.cluster,
            region: cli.region,
            stake_pool_deposit_stake_authority: cli.stake_pool_deposit_stake_authority,
            whitelist: cli.whitelist,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with a simpler configuration
    tracing_subscriber::fmt() // Use fully qualified path
        .with_max_level(Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .pretty()
        .init();

    info!("Logger initialized");

    // Load .env file before parsing so clap can read from it
    dotenv().ok();
    info!("Environment loaded");

    // Parse configuration from CLI args / environment
    let config = CrankerConfig::try_from(Cli::parse())?;

    info!("Configuration loaded successfully:");
    info!("RPC URL: {}", config.rpc_url);
    info!("Program ID: {}", config.program_id);
    info!("Payer: {}", config.payer.as_ref().pubkey()); // Signer trait now in scope
    info!("Interval: {}s", config.interval.as_secs());
    info!("Cluster: {}", config.cluster);
    info!("Region: {}", config.region);
    info!(
        "Stake Pool Deposit Stake Authority: {}",
        config.stake_pool_deposit_stake_authority
    );
    info!("Whitelist: {}", config.whitelist);

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
