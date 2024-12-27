use anchor_lang::prelude::*;

use crate::state::{AiDexPool, AiDexConfig};

#[event]
pub struct ProtocolFeeRateSetEvent {
    pub ai_dex_config: Pubkey,
    pub ai_dex_pool: Pubkey,
    pub config_authority: Pubkey,
    pub new_protocol_fee_rate: u16,
}

#[derive(Accounts)]
pub struct SetProtocolFeeRate<'info> {
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(mut, has_one = ai_dex_config)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,
}

/// Sets a new protocol fee rate for the AI DEX.
///
/// This function updates the protocol fee rate in the AI DEX configuration.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for setting the new protocol fee rate.
/// * `protocol_fee_rate` - The new protocol fee rate to be set.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the protocol fee rate is successfully updated,
/// or an `Err` if an error occurs.
pub fn set_protocol_fee_rate_handler(
    ctx: Context<SetProtocolFeeRate>,
    protocol_fee_rate: u16
) -> Result<()> {
    ctx
        .accounts
        .ai_dex_pool
        .load_mut()?
        .update_protocol_fee_rate(protocol_fee_rate)?;

    emit!(ProtocolFeeRateSetEvent {
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        config_authority: ctx.accounts.config_authority.key(),
        new_protocol_fee_rate: protocol_fee_rate,
    });

    Ok(())
}
