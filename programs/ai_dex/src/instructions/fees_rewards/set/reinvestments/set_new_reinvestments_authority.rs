use anchor_lang::prelude::*;

use crate::state::AiDexReinvestments;

#[event]
pub struct ReinvestmentNewAuthoritySetEvent {
    pub reinvestments_account: Pubkey,
    pub reinvestments_authority: Pubkey,
    pub new_reinvestments_authority: Pubkey,
}

#[derive(Accounts)]
pub struct SetNewReinvestmentAuthority<'info> {
    #[account(mut)]
    pub reinvestments_account: Account<'info, AiDexReinvestments>,

    #[account(address = reinvestments_account.reinvestments_authority)]
    pub reinvestments_authority: Signer<'info>,
}

pub fn set_new_reinvestments_authority_handler(
    ctx: Context<SetNewReinvestmentAuthority>,
    new_reinvestments_authority: Pubkey,
) -> Result<()> {
    ctx
        .accounts
        .reinvestments_account
        .update_reinvestments_authority(new_reinvestments_authority)?;

    emit!(ReinvestmentNewAuthoritySetEvent {
        reinvestments_account: ctx.accounts.reinvestments_account.key(),
        reinvestments_authority: ctx.accounts.reinvestments_authority.key(),
        new_reinvestments_authority,
    });        

    Ok(())
}
