use anchor_lang::prelude::*;

use crate::state::*;

#[event]
pub struct ReinvestmentsAuthorityInitializedEvent {
    pub reinvestments_account: Pubkey,
    pub super_admin_authority: Pubkey,
    pub reinvestments_authority: Pubkey,
    pub default_reinvestment_fee_rate: u16,
}

#[derive(Accounts)]
#[instruction(reinvestments_authority: Pubkey)]
pub struct InitializeReinvestmentsAuthority<'info> {
    /// SuperAdmin account that stores the current super admin's public key.
    #[account(constraint = super_admin_account.super_admin == super_admin_authority.key())]
    pub super_admin_account: Account<'info, SuperAdmin>,

    /// Signer must be the current super admin.
    #[account(mut)]
    pub super_admin_authority: Signer<'info>,

    #[account(
        init,
        payer = super_admin_authority, space = AiDexReinvestments::LEN,
        seeds = [
            b"reinvestments".as_ref(),
            reinvestments_authority.key().as_ref(),
        ],
        bump,
    )]
    pub reinvestments_account: Account<'info, AiDexReinvestments>,

    pub system_program: Program<'info, System>,
}

/// Initializes the reinvestments authority for the protocol.
pub fn initialize_reinvestments_handler(
    ctx: Context<InitializeReinvestmentsAuthority>,
    reinvestments_authority: Pubkey,
    default_reinvestment_fee_rate: u16,
) -> Result<()> {
    let reinvestments_account = &mut ctx.accounts.reinvestments_account;

    reinvestments_account.initialize(
        reinvestments_authority,
        default_reinvestment_fee_rate,
    )?;

    emit!(ReinvestmentsAuthorityInitializedEvent {
        reinvestments_account: reinvestments_account.key(),
        super_admin_authority: ctx.accounts.super_admin_authority.key(),
        reinvestments_authority: reinvestments_authority,
        default_reinvestment_fee_rate,
    });
    
    Ok(())
}
