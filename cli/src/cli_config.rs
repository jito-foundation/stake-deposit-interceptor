use std::sync::Arc;

use solana_commitment_config::CommitmentConfig;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;

#[derive(Clone)]
pub struct CliConfig {
    /// The RPC endpoint URL
    pub rpc_url: String,

    /// The commitment level
    pub commitment: CommitmentConfig,

    /// Optional signer
    pub signer: Arc<Keypair>,

    /// Create a Squads multisig proposal instead of direct execution
    pub squads_proposal: bool,

    /// Squads multisig account address.
    /// Note: This is the Squads multisig account, NOT the vault PDA. The vault PDA will be derived from this
    /// multisig address and will act as the signing authority for the operation.
    pub squads_multisig: Option<Pubkey>,

    /// Vault index for the Squads multisig (default: 0)
    pub squads_vault_index: Option<u8>,

    /// Squads program ID (defaults to mainnet Squads v4 program)
    pub squads_program_id: Option<Pubkey>,
}
