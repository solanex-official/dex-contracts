use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use anchor_spl::metadata::Metadata;

use crate::constants::nft::ai_dex_nft_update_auth::ID as ADB_NFT_UPDATE_AUTH;
use crate::{state::*, util::mint_position_trade_batch_token_with_metadata_and_remove_authority};

#[event]
pub struct InitializePositionTradeBatchWithMetadataEvent {
    pub funder: Pubkey,
    pub position_trade_batch: Pubkey,
    pub position_trade_batch_mint: Pubkey,
    pub position_trade_batch_metadata: Pubkey,
    pub position_trade_batch_token_account_key: Pubkey,
    pub position_trade_batch_token_account_mint: Pubkey,
    pub position_trade_batch_owner: Pubkey,
    pub metadata_update_auth: Pubkey,
    pub token_program: Pubkey,
    pub associated_token_program: Pubkey,
    pub metadata_program: Pubkey,
    pub position_seed: u64,
}

#[derive(Accounts)]
#[instruction(position_seed: u64)]
pub struct InitializePositionTradeBatchWithMetadata<'info> {
    #[account(init,
        payer = funder,
        space = PositionTradeBatch::LEN,
        seeds = [b"position_trade_batch".as_ref(), position_trade_batch_mint.key().as_ref()],
        bump,
    )]
    pub position_trade_batch: Box<Account<'info, PositionTradeBatch>>,

    #[account(init,
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

    /// CHECK: checked via the Metadata CPI call
    #[account(mut)]
    pub position_trade_batch_metadata: UncheckedAccount<'info>,

    #[account(init,
        payer = funder,
        associated_token::mint = position_trade_batch_mint,
        associated_token::authority = position_trade_batch_owner,
    )]
    pub position_trade_batch_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: the account that will be the owner of the position trade batch can be arbitrary
    pub position_trade_batch_owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub funder: Signer<'info>,

    /// CHECK: checked via account constraints
    #[account(address = ADB_NFT_UPDATE_AUTH)]
    pub metadata_update_auth: UncheckedAccount<'info>,

    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    pub metadata_program: Program<'info, Metadata>,
}

/// Initializes a trade batch position with metadata.
///
/// This function handles the initialization of a trade batch position along with its metadata.
/// It first initializes the position trade batch with the provided mint key. Then, it mints
/// the position trade batch token with metadata and removes the authority.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for initializing the trade batch position with metadata.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the position is successfully initialized,
/// or an `Err` if an error occurs.
pub fn initialize_trade_batch_position_with_metadata_handler(
    ctx: Context<InitializePositionTradeBatchWithMetadata>,
    position_seed: u64,
) -> Result<()> {
    let position_trade_batch_mint = &ctx.accounts.position_trade_batch_mint;
    let position_trade_batch = &mut ctx.accounts.position_trade_batch;

    position_trade_batch.initialize(position_trade_batch_mint.key())?;

    let bump = ctx.bumps.position_trade_batch;

    mint_position_trade_batch_token_with_metadata_and_remove_authority(
        &ctx.accounts.funder,
        &ctx.accounts.position_trade_batch,
        position_trade_batch_mint,
        &ctx.accounts.position_trade_batch_token_account,
        &ctx.accounts.position_trade_batch_metadata,
        &ctx.accounts.metadata_update_auth,
        &ctx.accounts.metadata_program,
        &ctx.accounts.token_program,
        &ctx.accounts.system_program,
        &ctx.accounts.rent,
        &[
            b"position_trade_batch".as_ref(),
            position_trade_batch_mint.key().as_ref(),
            &[bump],
        ],
    )?;

    emit!(InitializePositionTradeBatchWithMetadataEvent {
        funder: ctx.accounts.funder.key(),
        position_trade_batch: ctx.accounts.position_trade_batch.key(),
        position_trade_batch_mint: ctx.accounts.position_trade_batch_mint.key(),
        position_trade_batch_metadata: ctx.accounts.position_trade_batch_metadata.key(),
        position_trade_batch_token_account_key: ctx.accounts.position_trade_batch_token_account.key(),
        position_trade_batch_token_account_mint: ctx.accounts.position_trade_batch_token_account.mint,
        position_trade_batch_owner: ctx.accounts.position_trade_batch_owner.key(),
        metadata_update_auth: ctx.accounts.metadata_update_auth.key(),
        token_program: ctx.accounts.token_program.key(),
        associated_token_program: ctx.accounts.associated_token_program.key(),
        metadata_program: ctx.accounts.metadata_program.key(),
        position_seed,
    });
    

    Ok(())
}
