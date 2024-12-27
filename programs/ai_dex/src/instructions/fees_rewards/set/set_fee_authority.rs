use anchor_lang::prelude::*;

use crate::state::AiDexConfig;

#[event]
pub struct FeeAuthorityUpdatedEvent {
    pub ai_dex_config: Pubkey,
    pub old_fee_authority: Pubkey,
    pub new_fee_authority: Pubkey,
}

#[derive(Accounts)]
pub struct SetFeeAuthority<'info> {
    #[account(mut)]
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,

    /// CHECK: the account that will be new authority can be arbitrary
    pub new_config_authority: UncheckedAccount<'info>,
}

/// Sets a new fee authority for the AI DEX configuration.
///
/// This function updates the fee authority in the AI DEX configuration. Only the current fee authority
/// has permission to invoke this instruction.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for setting the new fee authority.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the fee authority is successfully updated,
/// or an `Err` if an error occurs.
pub fn set_fee_authority_handler(
    ctx: Context<SetFeeAuthority>
) -> Result<()> {
    ctx
        .accounts
        .ai_dex_config
        .update_config_authority(ctx.accounts.new_config_authority.key());

    emit!(FeeAuthorityUpdatedEvent {
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        old_fee_authority: ctx.accounts.config_authority.key(),
        new_fee_authority: ctx.accounts.new_config_authority.key(),
    });        

    Ok(())
}
