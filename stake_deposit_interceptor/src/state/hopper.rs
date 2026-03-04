use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

/// Uninitialized, no-data account used to hold SOL for paying withdraw fee
/// Must be empty and uninitialized to be used as a payer or `transfer` instructions fail
#[derive(Debug)]
pub struct Hopper;

impl Hopper {
    /// Returns the seeds for the PDA
    pub fn seeds(whitelist: &Pubkey) -> Vec<Vec<u8>> {
        vec![b"hopper".to_vec(), whitelist.to_bytes().to_vec()]
    }

    /// Find the program address for the hopper account
    ///
    /// # Arguments
    /// - `program_id` - The program ID
    /// - `whitelist` - The whitelist PDA
    ///
    /// # Returns
    /// - `Pubkey` - The program address
    /// - `u8` - The bump seed
    /// - `Vec<Vec<u8>>` - The seeds used to generate the PDA
    pub fn find_program_address(
        program_id: &Pubkey,
        whitelist: &Pubkey,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(whitelist);
        let (address, bump) = Pubkey::find_program_address(
            &seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>(),
            program_id,
        );
        (address, bump, seeds)
    }

    /// Attempts to load the account, returning an error if it's not valid.
    ///
    /// # Arguments
    /// - `program_id` - The program ID
    /// - `account` - The account to load the configuration from
    /// - `whitelist` - The whitelist PDA
    /// - `expect_writable` - Whether the account should be writable
    ///
    /// # Returns
    /// - `Result<(), ProgramError>` - The result of the operation
    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        whitelist: &Pubkey,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(&solana_system_interface::program::id()) {
            msg!("Hopper account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }

        let expected_pda = Self::find_program_address(program_id, whitelist).0;

        if account.key.ne(&expected_pda) {
            msg!("Hopper account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }

        if expect_writable && !account.is_writable {
            msg!("Hopper account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }
}
