use anchor_lang::prelude::*;

use crate::state::AiDexConfig;

#[event]
pub struct DefaultProtocolFeeRateSetEvent {
    pub ai_dex_config: Pubkey,
    pub config_authority: Pubkey,
    pub new_default_protocol_fee_rate: u16,
}

#[derive(Accounts)]
pub struct SetDefaultProtocolFeeRate<'info> {
    #[account(mut)]
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,
}

/// Sets the default protocol fee rate for the AI DEX configuration.
///
/// This function updates the default protocol fee rate in the AI DEX configuration.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for setting the default protocol fee rate.
/// * `default_protocol_fee_rate` - The new default protocol fee rate to be set.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the default protocol fee rate is successfully updated,
/// or an `Err` if an error occurs.
pub fn set_default_protocol_fee_rate_handler(
    ctx: Context<SetDefaultProtocolFeeRate>,
    default_protocol_fee_rate: u16,
) -> Result<()> {
    ctx
        .accounts
        .ai_dex_config
        .update_default_protocol_fee_rate(default_protocol_fee_rate)?;

    emit!(DefaultProtocolFeeRateSetEvent {
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        config_authority: ctx.accounts.config_authority.key(),
        new_default_protocol_fee_rate: default_protocol_fee_rate,
    });        

    Ok(())
}
