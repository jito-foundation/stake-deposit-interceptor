use jito_bytemuck::Discriminator;

use crate::whitelist::Whitelist;

/// Discriminators for whitelist management accounts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhitelistManagementDiscriminator {
    Whitelist = 1,
}

impl Discriminator for Whitelist {
    const DISCRIMINATOR: u8 = WhitelistManagementDiscriminator::Whitelist as u8;
}
