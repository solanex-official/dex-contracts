use anchor_lang::prelude::*;
use crate::state::super_admin::SuperAdmin;

/// Accounts required for updating the SuperAdmin account.
#[derive(Accounts)]
pub struct UpdateSuperAdmin<'info> {
    #[account(mut)]
    pub super_admin_account: Account<'info, SuperAdmin>,

    #[account(address = super_admin_account.super_admin)]
    pub super_admin_address: Signer<'info>,

    /// CHECK: the account that will be new authority can be arbitrary
    pub new_super_admin_address: UncheckedAccount<'info>,
}

/// Updates the SuperAdmin account with a new public key.
///
/// This function allows the current super admin to authorize a new account as the super admin. 
/// It ensures that only the current super admin has the authority to perform the update.
///
/// # Arguments
///
/// * `ctx` - The context containing all the required accounts. 
/// This includes the current super admin's signature and the new super admin's address.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the update is successful,
/// or an `Err` if an error occurs.
pub fn update_super_admin_handler(
    ctx: Context<UpdateSuperAdmin>,
) -> Result<()> {
    let super_admin_account = &mut ctx.accounts.super_admin_account;

    // Update the super admin
    super_admin_account.update_super_admin(ctx.accounts.new_super_admin_address.key());

    Ok(())
}