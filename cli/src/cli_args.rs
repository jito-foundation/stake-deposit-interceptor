use std::path::PathBuf;

use clap::{Parser, Subcommand};
use solana_pubkey::Pubkey;

use crate::interceptor::StakeDepositInterceptorCommands;

#[derive(Parser)]
#[command(author, version, about = "A CLI for Interceptor operations", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<ProgramCommand>,

    #[arg(long, global = true, help = "Path to the configuration file")]
    pub config_file: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        default_value = "https://api.mainnet-beta.solana.com",
        help = "RPC URL to use"
    )]
    pub rpc_url: Option<String>,

    #[arg(long, global = true, help = "Commitment level")]
    pub commitment: Option<String>,

    #[arg(long, global = true, help = "Stake Deposit Interceptor program ID")]
    pub stake_deposit_interceptor_program_id: Option<String>,

    #[arg(long, global = true, help = "Filepath or URL to a keypair")]
    pub signer: Option<String>,

    #[arg(long, global = true, help = "Verbose mode")]
    pub verbose: bool,

    #[arg(
        long,
        global = true,
        default_value = "false",
        help = "This will print out the raw TX instead of running it"
    )]
    pub print_tx: bool,

    #[arg(
        long,
        global = true,
        default_value = "false",
        help = "This will print out account information in JSON format"
    )]
    pub print_json: bool,

    #[arg(
        long,
        global = true,
        default_value = "false",
        help = "This will print out account information in JSON format with reserved space"
    )]
    pub print_json_with_reserves: bool,

    /// Create a Squads multisig proposal instead of direct execution
    #[arg(long, env, global = true, default_value = "false")]
    pub squads_proposal: bool,

    /// Squads multisig account address.
    /// Note: This is the Squads multisig account, NOT the vault PDA. The vault PDA will be derived from this
    /// multisig address and will act as the signing authority for the operation.
    #[arg(
        long,
        global = true,
        env,
        default_value = "9eZbWiHsPRsxLSiHxzg2pkXsAuQMwAjQrda7C7e21Fw6"
    )]
    pub squads_multisig: Option<Pubkey>,

    /// Vault index for the Squads multisig (default: 0)
    #[arg(long, env, global = true, default_value = "0")]
    pub squads_vault_index: Option<u8>,

    /// Squads program ID (defaults to mainnet Squads v4 program)
    #[arg(
        long,
        global = true,
        env,
        default_value = "SMPLtSp7KUa95ZM8RfEt9vQkNo4fU6Y4ZCkxzgv2vQn"
    )]
    pub squads_program_id: Option<Pubkey>,
}

#[derive(Subcommand)]
pub enum ProgramCommand {
    /// Whitelist Management program commands
    StakeDepositInterceptor {
        #[command(subcommand)]
        action: StakeDepositInterceptorCommands,
    },
}
