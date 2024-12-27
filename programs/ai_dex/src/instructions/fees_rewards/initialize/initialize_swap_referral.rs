use anchor_lang::prelude::*;
use crate::state::{AiDexConfig, SwapReferral};

#[event]
pub struct SwapReferralInitialized {
    pub config_account: Pubkey,
    pub referrer: Pubkey,
    pub referral_code: String,
    pub swap_referral: Pubkey,
}

#[derive(Accounts)]
#[instruction(referral_code: String)]
pub struct InitializeSwapReferral<'info> {
    pub config_account: Box<Account<'info, AiDexConfig>>,
    
    #[account(
        init, 
        payer = referrer, 
        space = SwapReferral::LEN, 
        seeds = [
            b"swap-referral".as_ref(),
            referrer.key().as_ref(),
            referral_code.as_ref(),
        ],
        bump
    )]
    pub swap_referral_account: Account<'info, SwapReferral>,

    #[account(mut)]
    pub referrer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_swap_referral_handler(
    ctx: Context<InitializeSwapReferral>,
    referral_code: String,
) -> Result<()> {
    let swap_referral = &mut ctx.accounts.swap_referral_account;
    swap_referral.initialize_swap_referral(
        ctx.bumps.swap_referral_account,
        ctx.accounts.referrer.key(),
        &referral_code,
    )?;

    emit!(SwapReferralInitialized {
        config_account: *ctx.accounts.config_account.to_account_info().key,
        referrer: ctx.accounts.referrer.key(),
        referral_code,
        swap_referral: *swap_referral.to_account_info().key,
    });
    Ok(())
}
