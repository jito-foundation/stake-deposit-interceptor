use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::ShankAccount;
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_program_log::log;
use solana_pubkey::Pubkey;

/// Sentinel value representing an empty/unset address slot
pub const EMPTY_ADDRESS: Pubkey = Pubkey::new_from_array([0u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct Whitelist {
    /// Whitelisted signers (read by the Interceptor for DepositStakeWhitelisted)
    /// Empty slots are represented by EMPTY_ADDRESS (all zeros)
    pub whitelist: [Pubkey; 64],

    /// 1-of-N admin set. Any single admin can manage the whitelist.
    /// Empty slots are represented by EMPTY_ADDRESS (all zeros)
    pub admins: [Pubkey; 8],

    /// Base keypair used to derive this PDA
    pub base: Pubkey,

    /// ( Optional )
    pub total_stake_deposited: PodU64,

    /// ( Optional )
    pub total_stake_withdrawn: PodU64,

    /// ( Optional )
    pub total_withdrawal_fees: PodU64,

    /// Bump of the PDA
    pub bump: u8,

    // More tracking as nessecary ( Optional )...
    /// Reserved for future use
    pub _padding: [u8; 512],
}

impl Default for Whitelist {
    fn default() -> Self {
        Self {
            whitelist: [EMPTY_ADDRESS; 64],
            admins: [EMPTY_ADDRESS; 8],
            base: Pubkey::default(),
            total_stake_deposited: PodU64::from(0),
            total_stake_withdrawn: PodU64::from(0),
            total_withdrawal_fees: PodU64::from(0),
            bump: 0,
            _padding: [0; 512],
        }
    }
}

impl Whitelist {
    pub const LEN: usize = size_of::<Self>();

    /// Returns the seeds for the PDA
    #[inline(always)]
    pub fn seeds(base: &Pubkey) -> Vec<Vec<u8>> {
        vec![b"whitelist".to_vec(), base.to_bytes().to_vec()]
    }

    /// Find the program address for the global configuration account
    ///
    /// # Arguments
    /// * `program_id` - The program ID
    /// # Returns
    /// * `Pubkey` - The program address
    /// * `u8` - The bump seed
    /// * `Vec<Vec<u8>>` - The seeds used to generate the PDA
    #[inline(always)]
    pub fn find_program_address(program_id: &Pubkey, base: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(base);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);

        (pda, bump, seeds)
    }

    /// Attempts to load the account as [`Config`], returning an error if it's not valid.
    ///
    /// # Arguments
    /// * `program_id` - The program ID
    /// * `account` - The account to load the configuration from
    /// * `expect_writable` - Whether the account should be writable
    ///
    /// # Returns
    /// * `Result<(), ProgramError>` - The result of the operation
    #[inline(always)]
    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        base: &Pubkey,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(program_id) {
            log!("Config account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.data_is_empty() {
            log!("Config account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !account.is_writable {
            log!("Config account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            log!("Config account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if account
            .key
            .ne(&Self::find_program_address(program_id, base).0)
        {
            log!("Config account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    /// Check if an address slot is empty
    #[inline(always)]
    pub fn is_empty_address(addr: &Pubkey) -> bool {
        addr == &EMPTY_ADDRESS
    }

    #[inline(always)]
    pub fn set_whitelist(&mut self, whitelist: [Pubkey; 64]) {
        self.whitelist = whitelist;
    }

    #[inline(always)]
    pub fn set_admins(&mut self, admins: [Pubkey; 8]) {
        self.admins = admins;
    }

    #[inline(always)]
    pub fn add_admin(&mut self, admin: Pubkey) {
        for a in self.admins.iter_mut() {
            if *a == EMPTY_ADDRESS {
                *a = admin;
                break;
            }
        }
    }

    #[inline(always)]
    pub fn remove_admin(
        &mut self,
        admin: &Pubkey,
        admin_to_remove: &Pubkey,
    ) -> Result<(), ProgramError> {
        if admin == admin_to_remove {
            return Err(ProgramError::InvalidAccountData);
        }

        for a in self.admins.iter_mut() {
            if *a == *admin_to_remove {
                *a = EMPTY_ADDRESS;
                break;
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn add_to_whitelist(&mut self, signer_to_add: Pubkey) {
        for a in self.whitelist.iter_mut() {
            if *a == EMPTY_ADDRESS {
                *a = signer_to_add;
                break;
            }
        }
    }

    #[inline(always)]
    pub fn remove_from_whitelist(&mut self, signer_to_remove: Pubkey) {
        for a in self.whitelist.iter_mut() {
            if *a == signer_to_remove {
                *a = EMPTY_ADDRESS;
                break;
            }
        }
    }

    #[inline(always)]
    pub fn set_base(&mut self, base: Pubkey) {
        self.base = base;
    }

    #[inline(always)]
    pub fn total_stake_deposited(&self) -> u64 {
        self.total_stake_deposited.into()
    }

    #[inline(always)]
    pub fn set_total_stake_deposited(&mut self, total_stake_deposited: u64) {
        self.total_stake_deposited = PodU64::from(total_stake_deposited);
    }

    #[inline(always)]
    pub fn total_stake_withdrawn(&self) -> u64 {
        self.total_stake_withdrawn.into()
    }

    #[inline(always)]
    pub fn set_total_stake_withdrawn(&mut self, total_stake_withdrawn: u64) {
        self.total_stake_withdrawn = PodU64::from(total_stake_withdrawn);
    }

    #[inline(always)]
    pub fn total_withdrawal_fees(&self) -> u64 {
        self.total_withdrawal_fees.into()
    }

    #[inline(always)]
    pub fn set_bump(&mut self, bump: u8) {
        self.bump = bump;
    }

    #[inline(always)]
    pub fn check_admin(&self, admin: &Pubkey) -> Result<(), ProgramError> {
        for a in self.admins.iter() {
            if a.ne(&EMPTY_ADDRESS) && a.eq(admin) {
                return Ok(());
            }
        }

        Err(ProgramError::InvalidAccountData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_address(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    #[test]
    fn test_default() {
        let wl = Whitelist::default();
        assert!(wl.whitelist.iter().all(|a| *a == EMPTY_ADDRESS));
        assert!(wl.admins.iter().all(|a| *a == EMPTY_ADDRESS));
        assert_eq!(wl.base, Pubkey::default());
        assert_eq!(wl.total_stake_deposited, PodU64::from(0));
        assert_eq!(wl.total_stake_withdrawn, PodU64::from(0));
        assert_eq!(wl.total_withdrawal_fees, PodU64::from(0));
        assert_eq!(wl.bump, 0);
        assert!(wl._padding.iter().all(|&b| b == 0));
    }

    // --- is_empty_address ---

    #[test]
    fn test_is_empty_address() {
        assert!(Whitelist::is_empty_address(&EMPTY_ADDRESS));
        assert!(!Whitelist::is_empty_address(&make_address(1)));
    }

    #[test]
    fn test_add_admin() {
        let mut wl = Whitelist::default();
        wl.add_admin(make_address(1));
        assert_eq!(wl.admins[0], make_address(1));
        assert!(wl.admins[1..].iter().all(|a| *a == EMPTY_ADDRESS));
    }

    #[test]
    fn test_add_multiple_admins() {
        let mut wl = Whitelist::default();
        for i in 1..=8 {
            wl.add_admin(make_address(i));
        }
        for i in 0..8 {
            assert_eq!(wl.admins[i], make_address(i as u8 + 1));
        }
    }

    #[test]
    fn test_add_admin_when_full_does_not_panic() {
        let mut wl = Whitelist::default();
        for i in 1..=8 {
            wl.add_admin(make_address(i));
        }
        // 9th add should silently do nothing
        wl.add_admin(make_address(99));
        assert!(wl.admins.iter().all(|a| *a != make_address(99)));
    }

    #[test]
    fn test_check_admin_found() {
        let mut wl = Whitelist::default();
        wl.add_admin(make_address(5));
        assert!(wl.check_admin(&make_address(5)).is_ok());
    }

    #[test]
    fn test_check_admin_not_found() {
        let wl = Whitelist::default();
        assert!(wl.check_admin(&make_address(5)).is_err());
    }

    #[test]
    fn test_check_admin_rejects_empty_address() {
        let wl = Whitelist::default();
        // All slots are EMPTY_ADDRESS; check_admin should still reject it
        assert!(wl.check_admin(&EMPTY_ADDRESS).is_err());
    }

    #[test]
    fn test_remove_admin() {
        let mut wl = Whitelist::default();
        wl.add_admin(make_address(1));
        wl.add_admin(make_address(2));
        assert!(wl.check_admin(&make_address(1)).is_ok());

        wl.remove_admin(&make_address(2), &make_address(1)).unwrap();
        assert!(wl.check_admin(&make_address(1)).is_err());
        assert_eq!(wl.admins[0], EMPTY_ADDRESS);
    }

    #[test]
    fn test_remove_admin_self_removal_fails() {
        let mut wl = Whitelist::default();
        wl.add_admin(make_address(1));
        // Admin cannot remove themselves
        assert!(wl.remove_admin(&make_address(1), &make_address(1)).is_err());
        // Admin should still be present
        assert!(wl.check_admin(&make_address(1)).is_ok());
    }

    #[test]
    fn test_remove_admin_not_present() {
        let mut wl = Whitelist::default();
        wl.add_admin(make_address(1));
        // Removing a non-existent admin should not affect existing ones
        wl.remove_admin(&make_address(1), &make_address(99))
            .unwrap();
        assert!(wl.check_admin(&make_address(1)).is_ok());
    }

    #[test]
    fn test_remove_first_matching_admin_only() {
        let mut wl = Whitelist::default();
        // Manually place the same address in two slots
        wl.admins[0] = make_address(1);
        wl.admins[1] = make_address(1);
        wl.admins[2] = make_address(2);
        wl.remove_admin(&make_address(2), &make_address(1)).unwrap();
        // Only the first occurrence should be removed
        assert_eq!(wl.admins[0], EMPTY_ADDRESS);
        assert_eq!(wl.admins[1], make_address(1));
    }

    #[test]
    fn test_set_admins() {
        let mut wl = Whitelist::default();
        let mut admins = [EMPTY_ADDRESS; 8];
        admins[0] = make_address(10);
        admins[3] = make_address(20);
        wl.set_admins(admins);
        assert_eq!(wl.admins[0], make_address(10));
        assert_eq!(wl.admins[3], make_address(20));
    }

    #[test]
    fn test_add_to_whitelist() {
        let mut wl = Whitelist::default();
        wl.add_to_whitelist(make_address(1));
        assert_eq!(wl.whitelist[0], make_address(1));
        assert!(wl.whitelist[1..].iter().all(|a| *a == EMPTY_ADDRESS));
    }

    #[test]
    fn test_add_multiple_to_whitelist() {
        let mut wl = Whitelist::default();
        for i in 1..=64 {
            wl.add_to_whitelist(make_address(i));
        }
        for i in 0..64 {
            assert_eq!(wl.whitelist[i], make_address(i as u8 + 1));
        }
    }

    #[test]
    fn test_add_to_whitelist_when_full_does_not_panic() {
        let mut wl = Whitelist::default();
        for i in 1..=64 {
            wl.add_to_whitelist(make_address(i));
        }
        // 65th add should silently do nothing
        wl.add_to_whitelist(make_address(200));
        assert!(wl.whitelist.iter().all(|a| *a != make_address(200)));
    }

    #[test]
    fn test_remove_from_whitelist() {
        let mut wl = Whitelist::default();
        wl.add_to_whitelist(make_address(1));
        wl.remove_from_whitelist(make_address(1));
        assert_eq!(wl.whitelist[0], EMPTY_ADDRESS);
    }

    #[test]
    fn test_remove_from_whitelist_not_present() {
        let mut wl = Whitelist::default();
        wl.add_to_whitelist(make_address(1));
        wl.remove_from_whitelist(make_address(99));
        assert_eq!(wl.whitelist[0], make_address(1));
    }

    #[test]
    fn test_remove_first_matching_whitelist_only() {
        let mut wl = Whitelist::default();
        wl.whitelist[0] = make_address(1);
        wl.whitelist[1] = make_address(1);
        wl.remove_from_whitelist(make_address(1));
        assert_eq!(wl.whitelist[0], EMPTY_ADDRESS);
        assert_eq!(wl.whitelist[1], make_address(1));
    }

    #[test]
    fn test_set_whitelist() {
        let mut wl = Whitelist::default();
        let mut list = [EMPTY_ADDRESS; 64];
        list[0] = make_address(10);
        list[63] = make_address(20);
        wl.set_whitelist(list);
        assert_eq!(wl.whitelist[0], make_address(10));
        assert_eq!(wl.whitelist[63], make_address(20));
    }

    #[test]
    fn test_set_base() {
        let mut wl = Whitelist::default();
        wl.set_base(make_address(42));
        assert_eq!(wl.base, make_address(42));
    }

    #[test]
    fn test_set_bump() {
        let mut wl = Whitelist::default();
        wl.set_bump(255);
        assert_eq!(wl.bump, 255);
    }

    #[test]
    fn test_set_total_stake_deposited() {
        let mut wl = Whitelist::default();
        wl.set_total_stake_deposited(1_000_000);
        assert_eq!(wl.total_stake_deposited, PodU64::from(1_000_000));
    }

    #[test]
    fn test_set_receive() {
        let mut wl = Whitelist::default();
        wl.set_total_stake_withdrawn(500_000);
        assert_eq!(wl.total_stake_withdrawn, PodU64::from(500_000));
    }

    #[test]
    fn test_seeds() {
        let base = make_address(7);
        let seeds = Whitelist::seeds(&base);
        assert_eq!(seeds.len(), 2);
        assert_eq!(seeds[0], b"whitelist".to_vec());
        assert_eq!(seeds[1], base.to_bytes().to_vec());
    }

    #[test]
    fn test_find_program_address_deterministic() {
        let program_id = make_address(1);
        let base = make_address(2);
        let (pda1, bump1, seeds1) = Whitelist::find_program_address(&program_id, &base);
        let (pda2, bump2, seeds2) = Whitelist::find_program_address(&program_id, &base);
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
        assert_eq!(seeds1, seeds2);
    }

    #[test]
    fn test_find_program_address_different_bases_differ() {
        let program_id = make_address(1);
        let (pda1, _, _) = Whitelist::find_program_address(&program_id, &make_address(2));
        let (pda2, _, _) = Whitelist::find_program_address(&program_id, &make_address(3));
        assert_ne!(pda1, pda2);
    }

    #[test]
    fn test_len_matches_struct_size() {
        assert_eq!(Whitelist::LEN, core::mem::size_of::<Whitelist>());
    }

    #[test]
    fn test_add_admin_reuses_removed_slot() {
        let mut wl = Whitelist::default();
        wl.add_admin(make_address(1));
        wl.add_admin(make_address(2));
        wl.remove_admin(&make_address(2), &make_address(1)).unwrap();
        // Adding a new admin should fill the first empty slot (index 0)
        wl.add_admin(make_address(3));
        assert_eq!(wl.admins[0], make_address(3));
        assert_eq!(wl.admins[1], make_address(2));
    }

    #[test]
    fn test_add_to_whitelist_reuses_removed_slot() {
        let mut wl = Whitelist::default();
        wl.add_to_whitelist(make_address(1));
        wl.add_to_whitelist(make_address(2));
        wl.remove_from_whitelist(make_address(1));
        wl.add_to_whitelist(make_address(3));
        assert_eq!(wl.whitelist[0], make_address(3));
        assert_eq!(wl.whitelist[1], make_address(2));
    }
}
