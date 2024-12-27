use anchor_lang::prelude::*;

/// The SuperAdmin account which holds the super admin's public key.
#[account]
pub struct SuperAdmin {
    pub super_admin: Pubkey, // Storing the super admin's public key
}

impl SuperAdmin {
    // Define the length of the account (32 bytes for `Pubkey` + 8 for discriminator).
    pub const LEN: usize = 32 + 8;

    pub fn initialize(&mut self, super_admin: Pubkey) {
        self.super_admin = super_admin;
    }

    pub fn update_super_admin(&mut self, super_admin: Pubkey) {
        self.super_admin = super_admin;
    }
}
