use std::{str::FromStr, sync::Arc};

use clap::Parser;
use env_logger::Env;
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey;
use stake_deposit_interceptor_cli::{
    cli_args::{Cli, ProgramCommand},
    cli_config::CliConfig,
    cli_signer::CliSigner,
    interceptor_handler::StakeDepositInterceptorCliHandler,
};
use stake_deposit_interceptor_client::programs::STAKE_DEPOSIT_INTERCEPTOR_ID;

pub fn get_cli_config(args: &Cli) -> Result<CliConfig, anyhow::Error> {
    let signer_path = &args.signer.clone().unwrap();
    let signer = CliSigner::new_keypair_from_path(signer_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair: {e}"))?;
    // let signer = match &args.signer {
    //     Some(path) => {
    //         let signer = if path == "ledger" {
    //             CliSigner::new_ledger()
    //         } else {
    //             CliSigner::new_keypair_from_path(path)
    //                 .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?
    //         };

    //         Some(Rc::new(signer))
    //     }
    //     _ => None,
    // };

    let cli_config = CliConfig {
        rpc_url: args
            .rpc_url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("rpc_url is required"))?,
        commitment: CommitmentConfig::from_str(
            args.commitment
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("commitment is required"))?,
        )?,
        signer: Arc::new(signer.keypair.unwrap()),
        squads_proposal: args.squads_proposal,
        squads_multisig: args.squads_multisig,
        squads_program_id: args.squads_program_id,
        squads_vault_index: args.squads_vault_index,
    };

    Ok(cli_config)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args: Cli = Cli::parse();

    let cli_config = get_cli_config(&args)?;

    let program_id = if let Some(program_id) = &args.stake_deposit_interceptor_program_id {
        Pubkey::from_str(program_id)?
    } else {
        STAKE_DEPOSIT_INTERCEPTOR_ID
    };

    match args.command.expect("Command not found") {
        ProgramCommand::StakeDepositInterceptor { action } => {
            StakeDepositInterceptorCliHandler::new(
                cli_config,
                program_id,
                args.print_tx,
                args.print_json,
                args.print_json_with_reserves,
            )
            .handle(action)
            .await?;
        }
    }

    Ok(())
}
