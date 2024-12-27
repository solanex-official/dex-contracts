use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::errors::ErrorCode;
use crate::{state::*, util::verify_position_trade_batch_authority};

#[event]
pub struct TradeBatchPositionClosedEvent {
    pub ai_dex_pool: Pubkey,
    pub trade_batch_index: u16,
    pub position_trade_batch: Pubkey,
    pub position_trade_batch_token_account: Pubkey,
    pub position_trade_batch_authority: Pubkey,
    pub trade_batch_position: Pubkey,
    pub receiver: Pubkey,
}

#[derive(Accounts)]
#[instruction(trade_batch_index: u16)]
pub struct CloseTradeBatchPosition<'info> {
    #[account(mut,
        close = receiver,
        seeds = [
            b"trade_batch_position".as_ref(),
            position_trade_batch.position_trade_batch_mint.key().as_ref(),
            trade_batch_index.to_string().as_bytes()
        ],
        bump,
    )]
    pub trade_batch_position: Account<'info, Position>,

    #[account(mut)]
    pub position_trade_batch: Box<Account<'info, PositionTradeBatch>>,

    #[account(
        constraint = position_trade_batch_token_account.mint == trade_batch_position.position_mint,
        constraint = position_trade_batch_token_account.mint == position_trade_batch.position_trade_batch_mint,
        constraint = position_trade_batch_token_account.amount == 1
    )]
    pub position_trade_batch_token_account: Box<Account<'info, TokenAccount>>,

    pub position_trade_batch_authority: Signer<'info>,
    
    /// CHECK: safe, for receiving rent only
    #[account(mut)]
    pub receiver: UncheckedAccount<'info>,
}

/// Closes a trade batch position if it is empty and the authority is verified.
///
/// This function handles the closure of a trade batch position. It first verifies
/// the authority of the position trade batch token account. If the position is not
/// empty, it returns an error. Otherwise, it proceeds to close the trade batch position.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for closing the trade batch position.
/// * `trade_batch_index` - The index of the trade batch to be closed.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the position is successfully closed,
/// or an `Err` if an error occurs.
///
/// # Errors
///
/// This function can return errors in the following cases:
/// * NonEmptyPositionCloseError if the position is not empty.
pub fn close_trade_batch_position_handler(ctx: Context<CloseTradeBatchPosition>, trade_batch_index: u16) -> Result<()> {
    let position_trade_batch = &mut ctx.accounts.position_trade_batch;

    // Allow delegation
    verify_position_trade_batch_authority(
        &ctx.accounts.position_trade_batch_token_account,
        &ctx.accounts.position_trade_batch_authority,
    )?;

    if !Position::is_position_empty(&ctx.accounts.trade_batch_position) {
        return Err(ErrorCode::NonEmptyPositionCloseError.into());
    }

    position_trade_batch.close_trade_batch_position(trade_batch_index)?;

    // Anchor will close the Position account

    emit!(TradeBatchPositionClosedEvent {
        ai_dex_pool: ctx.accounts.trade_batch_position.ai_dex_pool.key(),
        trade_batch_index,
        position_trade_batch: position_trade_batch.key(),
        position_trade_batch_token_account: ctx.accounts.position_trade_batch_token_account.key(),
        position_trade_batch_authority: ctx.accounts.position_trade_batch_authority.key(),
        trade_batch_position: ctx.accounts.trade_batch_position.key(),
        receiver: ctx.accounts.receiver.key(),
    });
    
    Ok(())
}
