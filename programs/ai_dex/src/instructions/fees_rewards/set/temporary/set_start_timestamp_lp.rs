use anchor_lang::prelude::*;

use crate::state::{AiDexPool, AiDexConfig};

#[event]
pub struct StartTimestampLpSetEvent {
    pub ai_dex_pool: Pubkey,
    pub ai_dex_config: Pubkey,
    pub config_authority: Pubkey,
    pub old_timestamp: u64,
    pub new_timestamp: u64,
}

#[derive(Accounts)]
pub struct SetTimestamp<'info> {
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(mut, has_one = ai_dex_config)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,
}

pub fn set_start_timestamp_lp_handler(
    ctx: Context<SetTimestamp>,
    new_timestamp: u64
) -> Result<()> {
    let mut ai_dex_pool = ctx.accounts.ai_dex_pool.load_mut()?;
    // Log the current fee rate before updating
    let old_timestamp = ai_dex_pool.start_timestamp_lp;
    
    ai_dex_pool.update_start_timestamp_lp(new_timestamp);

    emit!(StartTimestampLpSetEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        config_authority: ctx.accounts.config_authority.key(),
        old_timestamp,
        new_timestamp,
    });    

    Ok(())
}
