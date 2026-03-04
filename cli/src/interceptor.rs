use clap::Subcommand;
use solana_pubkey::Pubkey;

/// The CLI handler for the stake deposit interceptor program
#[derive(Subcommand)]
pub enum StakeDepositInterceptorCommands {
    /// Initialize, get, and set the whitelist management struct
    Interceptor {
        #[command(subcommand)]
        action: StakeDepositInterceptorActions,
    },
}

#[derive(Subcommand)]
pub enum StakeDepositInterceptorActions {
    /// Create a stake deposit authority for a specific stake pool
    CreateStakeDepositAuthority {
        /// Stake pool address
        #[arg(long, short)]
        pool: Pubkey,

        /// Fee wallet that will own the token account(s) to collect any fees from the interceptor
        #[arg(long)]
        fee_wallet: Pubkey,

        /// Duration for which fees are applied by the interceptor
        #[arg(long)]
        cool_down_seconds: u64,

        /// The fee rate (in basis points) that will be charged at time 0 and linearly decay until cool_down_seconds has elapsed
        #[arg(long)]
        initial_fee_bps: u32,

        /// The authority address that has permissions to adjust authority, cool_down_seconds, and initial_fee_bps
        #[arg(long)]
        authority: Pubkey,

        /// SPL Stake Pool Program ID
        #[arg(long, default_value = "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy")]
        spl_stake_pool_program_id: Pubkey,
    },

    /// Create a stake deposit authority for a specific stake pool
    UpdateStakeDepositAuthority {
        /// stake_deposit_authority of the stake pool that will be deposited to
        #[arg(long)]
        stake_deposit_authority: Pubkey,

        /// Jito Whitelist Management Program ID
        #[arg(long, default_value = "Wh1tea995dSzf9q4bmUCPM8s6URjT1HWMrp771bLW7G")]
        jito_whitelist_management_program_id: Pubkey,
    },

    /// Deposit active stake account into the stake pool and receive a receipt to claim pool tokens later
    DepositStake {
        /// stake_deposit_authority of the stake pool that will be deposited to
        #[arg(long)]
        stake_deposit_authority: Pubkey,

        /// Stake address to join the pool
        #[arg(long)]
        stake_account: Pubkey,

        /// Withdraw authority for the stake account to be deposited. [default: cli config keypair]
        #[arg(long)]
        withdraw_authority: Pubkey,

        /// Pool token account to receive the referral fees for deposits. \
        ///                        Defaults to the token receiver.
        #[arg(long)]
        referrer: Option<Pubkey>,

        /// SPL Stake Pool Program ID
        #[arg(long, default_value = "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy")]
        spl_stake_pool_program_id: Pubkey,
    },

    /// List all deposit receipts with their status (active/expired)
    ListReceipts {
        /// Stake deposit interceptor program ID [default: known program ID]
        #[arg(long)]
        program_id: Option<Pubkey>,

        /// Filter by specific stake pool address
        #[arg(long)]
        stake_pool: Option<Pubkey>,

        /// Only show expired receipts
        #[arg(long)]
        show_expired_only: bool,

        /// Only show active receipts
        #[arg(long)]
        show_active_only: bool,
    },

    /// Claim pool tokens for a specific deposit receipt
    ClaimTokens {
        /// The deposit receipt PDA address
        #[arg(long)]
        receipt_address: Pubkey,

        /// Destination token account [default: owner's ATA]
        #[arg(long)]
        destination: Option<Pubkey>,

        /// Skip fee calculation if claiming after cooldown period
        #[arg(long)]
        after_cooldown: bool,

        /// Create the destination ATA if it doesn't exist
        #[arg(long)]
        create_ata: bool,
    },

    /// Get a stake deposit authority for a specific stake pool
    GetStakeDepositAuthority {
        /// stake_deposit_authority of the stake pool that will be deposited to
        #[arg(long)]
        stake_deposit_authority: Pubkey,
    },

    /// Deposit active stake account into the stake pool using a whitelisted signer.
    /// Automatically authorizes the stake account's staker and withdrawer to the stake deposit authority before depositing.
    DepositStakeWhitelisted {
        /// Whitelist address
        #[arg(long)]
        whitelist: Pubkey,

        /// Stake deposit authority address for the stake pool
        #[arg(long)]
        stake_deposit_authority: Pubkey,

        /// Stake account to deposit into the pool
        #[arg(long)]
        deposit_stake: Pubkey,

        /// Validator stake account in the pool
        #[arg(long)]
        validator_stake: Pubkey,

        /// SPL Stake Pool Program ID
        #[arg(long, default_value = "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy")]
        spl_stake_pool_program_id: Pubkey,
    },

    /// Withdraw stake from the stake pool using a whitelisted signer
    WithdrawStakeWhitelisted {
        /// Whitelist address
        #[arg(long)]
        whitelist: Pubkey,

        /// Stake deposit authority address for the stake pool
        #[arg(long)]
        stake_deposit_authority: Pubkey,

        /// Validator stake account to split from
        #[arg(long)]
        stake_split_from: Pubkey,

        /// Path to keypair file for the new stake account to receive the split
        #[arg(long)]
        stake_split_to: String,

        /// Authority of the user's stake account
        #[arg(long)]
        user_stake_authority: Pubkey,

        /// Account to receive the fee rebate
        #[arg(long)]
        fee_rebate_recipient: Pubkey,

        /// SPL Stake Pool Program ID
        #[arg(long, default_value = "SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy")]
        spl_stake_pool_program_id: Pubkey,

        /// Amount of pool tokens to withdraw
        #[arg(long)]
        amount: u64,
    },

    /// Fund hopper
    FundHopper {
        /// Whitelist address
        #[arg(long)]
        whitelist: Pubkey,

        /// Lamports
        #[arg(long)]
        lamports: u64,
    },

    /// Hopper Balance
    HopperBalance {
        /// Whitelist address
        #[arg(long)]
        whitelist: Pubkey,
    },
}
