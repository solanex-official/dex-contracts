use anchor_lang::prelude::*;

use crate::state::AiDexReinvestments;

#[event]
pub struct DefaultReinvestmentFeeRateSetEvent {
    pub reinvestments_account: Pubkey,
    pub reinvestments_authority: Pubkey,
    pub new_default_reinvestment_fee_rate: u16,
}

#[derive(Accounts)]
pub struct SetDefaultReinvestmentFeeRate<'info> {
    #[account(mut)]
    pub reinvestments_account: Account<'info, AiDexReinvestments>,

    #[account(address = reinvestments_account.reinvestments_authority)]
    pub reinvestments_authority: Signer<'info>,
}

pub fn set_default_reinvestment_fee_rate_handler(
    ctx: Context<SetDefaultReinvestmentFeeRate>,
    new_default_reinvestment_fee_rate: u16,
) -> Result<()> {
    ctx
        .accounts
        .reinvestments_account
        .update_default_reinvestment_fee_rate(new_default_reinvestment_fee_rate)?;

    emit!(DefaultReinvestmentFeeRateSetEvent {
        reinvestments_account: ctx.accounts.reinvestments_account.key(),
        reinvestments_authority: ctx.accounts.reinvestments_authority.key(),
        new_default_reinvestment_fee_rate,
    });        

    Ok(())
}
