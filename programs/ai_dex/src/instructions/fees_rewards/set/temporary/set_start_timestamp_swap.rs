use anchor_lang::prelude::*;
use crate::instructions::fees_rewards::set::set_start_timestamp_lp::SetTimestamp;

#[event]
pub struct StartTimestampSwapSetEvent {
    pub ai_dex_pool: Pubkey,
    pub ai_dex_config: Pubkey,
    pub config_authority: Pubkey,
    pub old_timestamp: u64,
    pub new_timestamp: u64,
}

pub fn set_start_timestamp_swap_handler(
    ctx: Context<SetTimestamp>,
    new_timestamp: u64
) -> Result<()> {
    let mut ai_dex_pool = ctx.accounts.ai_dex_pool.load_mut()?;
    // Log the current fee rate before updating
    let old_timestamp = ai_dex_pool.start_timestamp_swap;
    
    ai_dex_pool.update_start_timestamp_swap(new_timestamp);

    emit!(StartTimestampSwapSetEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        config_authority: ctx.accounts.config_authority.key(),
        old_timestamp,
        new_timestamp,
    });    

    Ok(())
}
