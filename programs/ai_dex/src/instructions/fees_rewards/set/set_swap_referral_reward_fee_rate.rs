use anchor_lang::prelude::*;

use crate::state::{AiDexConfig, SwapReferral};

#[event]
pub struct SwapReferralRewardFeeRateSetEvent {
    pub ai_dex_config: Pubkey,
    pub swap_referral_account: Pubkey,
    pub config_authority: Pubkey,
    pub new_swap_referral_reward_fee_rate: u16,
}

#[derive(Accounts)]
pub struct SetSwapReferralRewardFeeRate<'info> {
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(mut)]
    pub swap_referral_account: Account<'info, SwapReferral>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,
}

pub fn set_swap_referral_reward_fee_rate_handler(
    ctx: Context<SetSwapReferralRewardFeeRate>,
    swap_referral_reward_fee_rate: u16
) -> Result<()> {
    ctx
        .accounts
        .swap_referral_account
        .update_swap_reward_fee_rate(swap_referral_reward_fee_rate)?;

    emit!(SwapReferralRewardFeeRateSetEvent {
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        swap_referral_account: ctx.accounts.swap_referral_account.key(),
        config_authority: ctx.accounts.config_authority.key(),
        new_swap_referral_reward_fee_rate: swap_referral_reward_fee_rate,
    });

    Ok(())
}
