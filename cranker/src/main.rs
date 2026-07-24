use std::{process::Command, sync::Arc, time::Duration};

use clap::Parser;
use ::{
    dotenv::dotenv,
    solana_commitment_config::CommitmentConfig,
    solana_metrics::set_host_id,
    solana_sdk::{
        pubkey::Pubkey,
        signature::{read_keypair_file, Signer},
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

/// Strips path, query, and userinfo from an RPC URL so embedded API keys
/// never reach the logs.
fn redact_url(url: &str) -> String {
    match url.split_once("://") {
        Some((scheme, rest)) => {
            let host = rest
                .split(['/', '?', '#'])
                .next()
                .unwrap_or("")
                .rsplit('@')
                .next()
                .unwrap_or("");
            format!("{scheme}://{host}/<redacted>")
        }
        None => "<redacted>".to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .compact()
        .init();

    info!("Starting interceptor cranker service");

    let config = CrankerConfig::try_from(Cli::parse())?;

    info!("rpc_url={}", redact_url(&config.rpc_url));
    info!("program_id={}", config.program_id);
    info!("payer={}", config.payer.as_ref().pubkey());
    info!("interval_seconds={}", config.interval.as_secs());
    info!("cluster={}", config.cluster);
    info!("region={}", config.region);
    info!(
        "stake_pool_deposit_stake_authority={}",
        config.stake_pool_deposit_stake_authority
    );
    info!("whitelist={}", config.whitelist);

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

    let cranker = InterceptorCranker::new(config);
    info!("Starting cranker service");
    cranker.start().await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::redact_url;

    #[test]
    fn test_redact_url_strips_path_and_query() {
        assert_eq!(
            redact_url("https://mainnet.helius-rpc.com/?api-key=SECRET"),
            "https://mainnet.helius-rpc.com/<redacted>"
        );
        assert_eq!(
            redact_url("https://example.com/rpc/SECRET"),
            "https://example.com/<redacted>"
        );
    }

    #[test]
    fn test_redact_url_strips_userinfo() {
        assert_eq!(
            redact_url("https://SECRET@example.com/rpc"),
            "https://example.com/<redacted>"
        );
        assert_eq!(
            redact_url("https://user:pass@example.com:8899"),
            "https://example.com:8899/<redacted>"
        );
    }

    #[test]
    fn test_redact_url_keeps_host_and_port() {
        assert_eq!(
            redact_url("http://127.0.0.1:8899"),
            "http://127.0.0.1:8899/<redacted>"
        );
    }

    #[test]
    fn test_redact_url_without_scheme() {
        assert_eq!(redact_url("not a url"), "<redacted>");
    }
}
