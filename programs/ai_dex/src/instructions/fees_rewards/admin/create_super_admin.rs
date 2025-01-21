use anchor_lang::prelude::*;
use crate::state::super_admin::SuperAdmin;
use crate::errors::ErrorCode;

/// Account required for creating the SuperAdmin account.
#[derive(Accounts)]
pub struct CreateSuperAdmin<'info> {
    #[account(
        init, 
        payer = funder, 
        space = SuperAdmin::LEN, 
        seeds = [b"super-admin".as_ref()], 
        bump
    )]
    pub super_admin_account: Account<'info, SuperAdmin>,

    #[account(mut)]
    pub funder: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Initializes the super admin account with the provided public key.
///
/// This function is responsible for initializing the `SuperAdmin` account with the specified public key, 
/// which represents the super admin of the protocol. The initialization can only be done once.
///
/// # Arguments
///
/// * `ctx` - The context containing all the required accounts.
/// * `super_admin` - The public key of the account to be set as the super admin.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the initialization is successful,
/// or an `Err` if an error occurs.
pub fn create_super_admin_handler(
    ctx: Context<CreateSuperAdmin>,
    super_admin: Pubkey,
) -> Result<()> {
    // Validate the provided super_admin public key is not default/empty.
    if super_admin == Pubkey::default() {
        return Err(ErrorCode::EmptyAdminInput.into());
    }

    let super_admin_account = &mut ctx.accounts.super_admin_account;

    // Check if the super admin has already been initialized.
    if super_admin_account.super_admin != Pubkey::default() {
        return Err(ErrorCode::SuperAdminAlreadyInitialized.into());
    }

    // Initialize the super admin
    super_admin_account.initialize(super_admin);

    Ok(())
}
