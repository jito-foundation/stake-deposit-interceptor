use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StakeDepositInterceptorError {
    /// 0 : A signature was missing
    #[error("Signature missing")]
    SignatureMissing,
    /// 1 : Invalid seeds for PDA
    #[error("Invalid seeds")]
    InvalidSeeds,
    /// 2 : Account already in use
    #[error("Account already in use")]
    AlreadyInUse,
    /// 3 : Invalid StakePool
    #[error("StakePool does not match other inputs")]
    InvalidStakePool,
    /// 4 : Invalid StakePool Manager
    #[error("StakePool manager is invalid")]
    InvalidStakePoolManager,
    /// 5 : Invalid Authority
    #[error("Authority is invalid")]
    InvalidAuthority,
    /// 6 : Invalid StakePoolDepositStakeAuthority
    #[error("StakePoolDepositStakeAuthority key is invalid")]
    InvalidStakePoolDepositStakeAuthority,
    /// 7 : Invalid Vault account
    #[error("Vault ATA is invalid")]
    InvalidVault,
    /// 8 : Invalid Token program account
    #[error("Token program is invalid")]
    InvalidTokenProgram,
    /// 9 : Invalid DepositReceipt account
    #[error("DepositReceipt key is invalid")]
    InvalidDepositReceipt,
    /// 10 : Invalid DepositReceipt owner account
    #[error("DepositReceipt owner is invalid")]
    InvalidDepositReceiptOwner,
    /// 11 : Invalid fee token account
    #[error("Fee token account is invalid")]
    InvalidFeeTokenAccount,
    /// 12 : Invalid destination token account
    #[error("Destination token account is invalid")]
    InvalidDestinationTokenAccount,
    /// 13 : Cannot claim on behalf of owner until cool down has ended
    #[error("Only owner can claim during cool down period")]
    ActiveCooldown,
    /// 14 : Fee rate exceeds the max limit
    #[error("Fee rate exceeds the max limit")]
    InitialFeeRateMaxExceeded,
    /// 15 : Invalid pool mint
    #[error("Invalid pool mint")]
    InvalidPoolMint,
    /// 16 : Invalid stake-pool program
    #[error("StakePool program is invalid")]
    InvalidStakePoolProgram,

    /// 17 : Signer is not whitelisted
    #[error("Whitelisted signer is invalid")]
    InvalidWhitelistedSigner,

    /// 18 : Calculation failed
    #[error("CalculationFailure")]
    CalculationFailure,

    /// 18 : ArithmeticError
    #[error("ArithmeticError")]
    ArithmeticError,
}

impl From<StakeDepositInterceptorError> for ProgramError {
    fn from(value: StakeDepositInterceptorError) -> Self {
        Self::Custom(value as u32)
    }
}
