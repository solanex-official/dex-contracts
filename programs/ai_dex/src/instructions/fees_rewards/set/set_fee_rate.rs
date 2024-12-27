use anchor_lang::prelude::*;

use crate::state::{AiDexPool, AiDexConfig};

#[event]
pub struct FeeRateSetEvent {
    pub ai_dex_pool: Pubkey,
    pub ai_dex_config: Pubkey,
    pub config_authority: Pubkey,
    pub old_fee_rate: u16,
    pub new_fee_rate: u16,
}

#[derive(Accounts)]
pub struct SetFeeRate<'info> {
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(mut, has_one = ai_dex_config)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,
}

/// Sets a new fee rate for the AI DEX.
///
/// This function updates the fee rate in the AI DEX configuration.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for setting the new fee rate.
/// * `fee_rate` - The new fee rate to be set.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the fee rate is successfully updated,
/// or an `Err` if an error occurs.
pub fn set_fee_rate_handler(
    ctx: Context<SetFeeRate>,
    fee_rate: u16
) -> Result<()> {
    let mut ai_dex_pool = ctx.accounts.ai_dex_pool.load_mut()?;
    // Log the current fee rate before updating
    let old_fee_rate = ai_dex_pool.fee_rate;
    
    ai_dex_pool.update_fee_rate(fee_rate)?;

    emit!(FeeRateSetEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        config_authority: ctx.accounts.config_authority.key(),
        old_fee_rate,
        new_fee_rate: fee_rate,
    });    

    Ok(())
}
