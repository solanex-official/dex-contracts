use anchor_lang::prelude::*;

use crate::state::{FeeTier, AiDexConfig};

#[event]
pub struct DefaultFeeRateSetEvent {
    pub ai_dex_config: Pubkey,
    pub fee_tier_key: Pubkey,
    pub config_authority: Pubkey,
    pub previous_default_fee_rate: u16,
    pub new_default_fee_rate: u16,
}

#[derive(Accounts)]
pub struct SetDefaultFeeRate<'info> {
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(mut, has_one = ai_dex_config)]
    pub fee_tier: Account<'info, FeeTier>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,
}

/// Sets the default fee rate for a fee tier.
///
/// This function updates the default fee rate for a specified fee tier in the AI DEX configuration.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for setting the default fee rate.
/// * `default_fee_rate` - The new default fee rate to be set for the fee tier.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the default fee rate is successfully updated,
/// or an `Err` if an error occurs.
pub fn set_default_fee_rate_handler(ctx: Context<SetDefaultFeeRate>, default_fee_rate: u16) -> Result<()> {
    let old_default_fee_rate = ctx.accounts.fee_tier.default_fee_rate;
    ctx
        .accounts
        .fee_tier
        .update_default_fee_rate(default_fee_rate)?;

    emit!(DefaultFeeRateSetEvent {
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        fee_tier_key: ctx.accounts.fee_tier.key(),
        config_authority: ctx.accounts.config_authority.key(),
        previous_default_fee_rate: old_default_fee_rate,
        new_default_fee_rate: default_fee_rate,
    });
        
    Ok(())
}
