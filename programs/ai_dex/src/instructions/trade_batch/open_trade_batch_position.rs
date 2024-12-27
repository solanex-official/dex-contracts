use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::{state::*, util::verify_position_trade_batch_authority};

#[event]
pub struct TradeBatchPositionOpenedEvent {
    pub trade_batch_index: u16,
    pub position_trade_batch_key: Pubkey,
    pub position_trade_batch_mint: Pubkey,
    pub trade_batch_position_tick_lower_index: i32,
    pub trade_batch_position_tick_upper_index: i32,
    pub position_trade_batch_authority: Pubkey,
    pub position_trade_batch_token_account_key: Pubkey,
    pub position_trade_batch_token_account_mint: Pubkey,
    pub position_trade_batch_token_account_amount: u64,
    pub ai_dex_pool: Pubkey,
    pub funder: Pubkey,
    pub is_reinvestment_on: bool,
}

#[derive(Accounts)]
#[instruction(trade_batch_index: u16)]
pub struct OpenTradeBatchPosition<'info> {
    #[account(init,
        payer = funder,
        space = Position::LEN,
        seeds = [
            b"trade_batch_position".as_ref(),
            position_trade_batch.position_trade_batch_mint.key().as_ref(),
            trade_batch_index.to_string().as_bytes()
        ],
        bump,
    )]
    pub trade_batch_position: Box<Account<'info, Position>>,

    #[account(mut)]
    pub position_trade_batch: Box<Account<'info, PositionTradeBatch>>,

    #[account(
        constraint = position_trade_batch_token_account.mint == position_trade_batch.position_trade_batch_mint,
        constraint = position_trade_batch_token_account.amount == 1
    )]
    pub position_trade_batch_token_account: Box<Account<'info, TokenAccount>>,

    pub position_trade_batch_authority: Signer<'info>,

    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(mut)]
    pub funder: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Opens a trade batch position with specified tick indices.
///
/// This function handles the opening of a trade batch position. It first verifies the
/// authority of the position trade batch token account. Then, it opens the trade batch
/// position and sets the position with the specified tick indices.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for opening the trade batch position.
/// * `trade_batch_index` - The index of the trade batch to be opened.
/// * `tick_lower_index` - The lower tick index for the position.
/// * `tick_upper_index` - The upper tick index for the position.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the position is successfully opened,
/// or an `Err` if an error occurs.
pub fn open_trade_batch_position_handler(
    ctx: Context<OpenTradeBatchPosition>,
    trade_batch_index: u16,
    tick_lower_index: i32,
    tick_upper_index: i32,
    is_reinvestment_on: bool,
) -> Result<()> {
    let ai_dex = &ctx.accounts.ai_dex_pool;
    let position_trade_batch = &mut ctx.accounts.position_trade_batch;
    let position = &mut ctx.accounts.trade_batch_position;

    // Allow delegation
    verify_position_trade_batch_authority(
        &ctx.accounts.position_trade_batch_token_account,
        &ctx.accounts.position_trade_batch_authority,
    )?;

    position_trade_batch.open_trade_batch_position(trade_batch_index)?;

    position.open_position(
        ai_dex,
        position_trade_batch.position_trade_batch_mint,
        tick_lower_index,
        tick_upper_index,
        is_reinvestment_on,
    )?;

    emit!(TradeBatchPositionOpenedEvent {
        trade_batch_index,
        position_trade_batch_key: position_trade_batch.key(),
        position_trade_batch_mint: position_trade_batch.position_trade_batch_mint,
        trade_batch_position_tick_lower_index: tick_lower_index,
        trade_batch_position_tick_upper_index: tick_upper_index,
        position_trade_batch_authority: ctx.accounts.position_trade_batch_authority.key(),
        position_trade_batch_token_account_key: ctx.accounts.position_trade_batch_token_account.key(),
        position_trade_batch_token_account_mint: ctx.accounts.position_trade_batch_token_account.mint,
        position_trade_batch_token_account_amount: ctx.accounts.position_trade_batch_token_account.amount,
        ai_dex_pool: ai_dex.key(),
        funder: ctx.accounts.funder.key(),
        is_reinvestment_on,
    });

    Ok(())
}
