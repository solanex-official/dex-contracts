use anchor_lang::prelude::*;

use crate::state::AiDexConfig;

#[event]
pub struct DefaultSwapReferralRewardFeeRateSetEvent {
    pub ai_dex_config: Pubkey,
    pub config_authority: Pubkey,
    pub default_swap_referral_reward_fee_rate: u16,
}

#[derive(Accounts)]
pub struct SetDefaultSwapReferralRewardFeeRate<'info> {
    #[account(mut)]
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,
}

pub fn set_default_swap_referral_reward_fee_rate_handler(
    ctx: Context<SetDefaultSwapReferralRewardFeeRate>,
    default_swap_referral_reward_fee_rate: u16,
) -> Result<()> {
    ctx
        .accounts
        .ai_dex_config
        .update_default_swap_referral_reward_fee_rate(default_swap_referral_reward_fee_rate)?;

    emit!(DefaultSwapReferralRewardFeeRateSetEvent {
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        config_authority: ctx.accounts.config_authority.key(),
        default_swap_referral_reward_fee_rate,
    });        

    Ok(())
}
