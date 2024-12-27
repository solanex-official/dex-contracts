use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::errors::ErrorCode;
use crate::state::*;
use crate::util::burn_and_close_position_trade_batch_token;

#[event]
pub struct PositionTradeBatchDeletedEvent {
    pub position_trade_batch: Pubkey,
    pub position_trade_batch_mint: Pubkey,
    pub position_trade_batch_token_account_key: Pubkey,
    pub position_trade_batch_token_account_amount: u64,
    pub position_trade_batch_token_account_owner: Pubkey,
    pub position_trade_batch_owner: Pubkey,
    pub receiver: Pubkey,
    pub token_program: Pubkey,
}

#[derive(Accounts)]
pub struct DeletePositionTradeBatch<'info> {
    #[account(mut, close = receiver)]
    pub position_trade_batch: Account<'info, PositionTradeBatch>,

    #[account(mut, address = position_trade_batch.position_trade_batch_mint)]
    pub position_trade_batch_mint: Account<'info, Mint>,

    #[account(mut,
        constraint = position_trade_batch_token_account.mint == position_trade_batch.position_trade_batch_mint,
        constraint = position_trade_batch_token_account.owner == position_trade_batch_owner.key(),
        constraint = position_trade_batch_token_account.amount == 1,
    )]
    pub position_trade_batch_token_account: Box<Account<'info, TokenAccount>>,

    pub position_trade_batch_owner: Signer<'info>,

    /// CHECK: safe, for receiving rent only
    #[account(mut)]
    pub receiver: UncheckedAccount<'info>,

    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,
}

/// Deletes a trade batch position if it is deletable.
///
/// This function handles the deletion of a trade batch position. It first checks if the
/// position trade batch is deletable. If it is not deletable, it returns an error. Otherwise,
/// it proceeds to burn and close the position trade batch token.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for deleting the trade batch position.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the position is successfully deleted,
/// or an `Err` if an error occurs.
///
/// # Errors
///
/// This function can return errors in the following cases:
/// * NonDeletablePositionTradeBatchError if the position trade batch is not deletable.
pub fn delete_trade_batch_position_handler(ctx: Context<DeletePositionTradeBatch>) -> Result<()> {
    let position_trade_batch = &ctx.accounts.position_trade_batch;

    if !position_trade_batch.is_deletable() {
        return Err(ErrorCode::NonDeletablePositionTradeBatchError.into());
    }

    burn_and_close_position_trade_batch_token(
        &ctx.accounts.position_trade_batch_owner,
        &ctx.accounts.receiver,
        &ctx.accounts.position_trade_batch_mint,
        &ctx.accounts.position_trade_batch_token_account,
        &ctx.accounts.token_program,
    )?;

    emit!(PositionTradeBatchDeletedEvent {
        position_trade_batch: ctx.accounts.position_trade_batch.key(),
        position_trade_batch_mint: ctx.accounts.position_trade_batch_mint.key(),
        position_trade_batch_token_account_key: ctx.accounts.position_trade_batch_token_account.key(),
        position_trade_batch_token_account_amount: ctx.accounts.position_trade_batch_token_account.amount,
        position_trade_batch_token_account_owner: ctx.accounts.position_trade_batch_token_account.owner,
        position_trade_batch_owner: ctx.accounts.position_trade_batch_owner.key(),
        receiver: ctx.accounts.receiver.key(),
        token_program: ctx.accounts.token_program.key(),
    });
    
    Ok(())
}
