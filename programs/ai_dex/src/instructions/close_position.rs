use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::errors::ErrorCode;
use crate::state::*;
use crate::util::{burn_and_close_user_position_token, verify_position_authority};

#[event]
pub struct PositionClosedEvent {
    pub ai_dex_pool: Pubkey,
    pub position_authority: Pubkey,
    pub receiver: Pubkey,
    pub position_mint: Pubkey,
    pub position_token_account_key: Pubkey,
    pub position_token_account_amount: u64,
    pub position_token_account_mint: Pubkey,
    pub position: Pubkey,
}

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    pub position_authority: Signer<'info>,

    /// CHECK: safe, for receiving rent only
    #[account(mut)]
    pub receiver: UncheckedAccount<'info>,

    #[account(mut,
        close = receiver,
        seeds = [
            b"position".as_ref(),
            position_mint.key().as_ref()
        ],
        bump,
    )]
    pub position: Account<'info, Position>,

    #[account(mut, address = position.position_mint)]
    pub position_mint: Account<'info, Mint>,

    #[account(mut,
        constraint = position_token_account.amount == 1,
        constraint = position_token_account.mint == position.position_mint)]
    pub position_token_account: Box<Account<'info, TokenAccount>>,

    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,
}

pub fn close_position_handler(ctx: Context<ClosePosition>) -> Result<()> {
    verify_position_authority(
        &ctx.accounts.position_token_account,
        &ctx.accounts.position_authority,
    )?;

    if !Position::is_position_empty(&ctx.accounts.position) {
        return Err(ErrorCode::NonEmptyPositionCloseError.into());
    }

    burn_and_close_user_position_token(
        &ctx.accounts.position_authority,
        &ctx.accounts.receiver,
        &ctx.accounts.position_mint,
        &ctx.accounts.position_token_account,
        &ctx.accounts.token_program,
    )?;
    
    emit!(PositionClosedEvent {
        ai_dex_pool: ctx.accounts.position.ai_dex_pool.key(),
        position_authority: ctx.accounts.position_authority.key(),
        receiver: ctx.accounts.receiver.key(),
        position_mint: ctx.accounts.position_mint.key(),
        position_token_account_key: ctx.accounts.position_token_account.key(),
        position_token_account_amount: ctx.accounts.position_token_account.amount,
        position_token_account_mint: ctx.accounts.position_token_account.mint,
        position: ctx.accounts.position.key(),
    });
    
    Ok(())
}
