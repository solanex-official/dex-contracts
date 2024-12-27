use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

use crate::{state::*, util::mint_position_trade_batch_token_and_remove_authority};

#[event]
pub struct InitializeTradeBatchPositionEvent {
    pub position_trade_batch: Pubkey,
    pub position_trade_batch_mint: Pubkey,
    pub position_trade_batch_token_account_key: Pubkey,
    pub position_trade_batch_token_account_mint: Pubkey,
    pub position_trade_batch_owner: Pubkey,
    pub funder: Pubkey,
    pub position_seed: u64,
}

#[derive(Accounts)]
#[instruction(position_seed: u64)]
pub struct InitializePositionTradeBatch<'info> {
    #[account(
        init,
        payer = funder,
        space = PositionTradeBatch::LEN,
        seeds = [b"position_trade_batch".as_ref(), position_trade_batch_mint.key().as_ref()],
        bump,
    )]
    pub position_trade_batch: Box<Account<'info, PositionTradeBatch>>,

    #[account(
        init,
        payer = funder,
        mint::authority = position_trade_batch, // will be removed in the transaction
        mint::decimals = 0,
        seeds = [
            b"position_trade_batch".as_ref(),
            position_seed.to_string().as_bytes(),
        ],
        bump,
    )]
    pub position_trade_batch_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = funder,
        associated_token::mint = position_trade_batch_mint,
        associated_token::authority = position_trade_batch_owner,
    )]
    pub position_trade_batch_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: the account that will be the owner of the position trade batch can be arbitrary
    pub position_trade_batch_owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub funder: Signer<'info>,

    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

/// Initializes a trade batch position.
///
/// This function handles the initialization of a trade batch position. It first initializes
/// the position trade batch with the provided mint key. Then, it mints the position trade batch
/// token and removes the authority.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for initializing the trade batch position.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the position is successfully initialized,
/// or an `Err` if an error occurs.
pub fn initialize_trade_batch_position_handler(
    ctx: Context<InitializePositionTradeBatch>,
    position_seed: u64,
) -> Result<()> {
    let position_trade_batch_mint = &ctx.accounts.position_trade_batch_mint;
    let position_trade_batch = &mut ctx.accounts.position_trade_batch;

    position_trade_batch.initialize(position_trade_batch_mint.key())?;

    let bump = ctx.bumps.position_trade_batch;

    mint_position_trade_batch_token_and_remove_authority(
        &ctx.accounts.position_trade_batch,
        position_trade_batch_mint,
        &ctx.accounts.position_trade_batch_token_account,
        &ctx.accounts.token_program,
        &[
            b"position_trade_batch".as_ref(),
            position_trade_batch_mint.key().as_ref(),
            &[bump],
        ],
    )?;

    emit!(InitializeTradeBatchPositionEvent {
        position_trade_batch: ctx.accounts.position_trade_batch.key(),
        position_trade_batch_mint: position_trade_batch_mint.key(),
        position_trade_batch_token_account_key: ctx.accounts.position_trade_batch_token_account.key(),
        position_trade_batch_token_account_mint: ctx.accounts.position_trade_batch_token_account.mint,
        position_trade_batch_owner: ctx.accounts.position_trade_batch_owner.key(),
        funder: ctx.accounts.funder.key(),
        position_seed,
    });

    Ok(())
}
