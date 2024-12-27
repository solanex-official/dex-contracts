use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    errors::ErrorCode,
    state::AiDexPool,
    util::is_supported_token_mint
};

#[event]
pub struct RewardInitializedEvent {
    pub reward_index: u8,
    pub ai_dex_pool: Pubkey,
    pub reward_authority: Pubkey,
    pub funder: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_vault: Pubkey,
}

#[derive(Accounts)]
#[instruction(reward_index: u8)]
pub struct InitializeReward<'info> {
    #[account(mut)]
    pub reward_authority: Signer<'info>,

    #[account(mut)]
    pub funder: Signer<'info>,

    #[account(mut)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    pub reward_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = funder,
        token::token_program = reward_token_program,
        token::mint = reward_mint,
        token::authority = ai_dex_pool,
        seeds = [
            b"reward_vault",
            reward_mint.to_account_info().key.as_ref(),
            reward_index.to_string().as_bytes(),
            ai_dex_pool.to_account_info().key.as_ref(),
        ],
        bump,
    )]
    pub reward_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(constraint = reward_token_program.key() == reward_mint.to_account_info().owner.clone())]
    pub reward_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Initializes a reward in the protocol.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts and programs required for the operation.
/// * `reward_index` - The index of the reward to be initialized.
///
/// # Returns
///
/// * `Result<()>` - Returns an Ok result if the operation is successful, otherwise returns an error.
///
/// # Errors
///
/// * `ErrorCode::UnsupportedTokenMintError` - If the token mint is not supported.
pub fn initialize_reward_handler(ctx: Context<InitializeReward>, reward_index: u8) -> Result<()> {
    let ai_dex = &mut ctx.accounts.ai_dex_pool.load_mut()?;

    // Ensure the reward_index is valid
    if reward_index as usize >= ai_dex.reward_infos.len() {
        return Err(ErrorCode::InvalidRewardIndexError.into());
    }

    // Check if the reward_authority matches the authority in reward_infos
    if ctx.accounts.reward_authority.key() != ai_dex.reward_infos[reward_index as usize].authority {
        return Err(ErrorCode::InvalidRewardAuthorityError.into());
    }

    if !is_supported_token_mint(&ctx.accounts.reward_mint).unwrap() {
        return Err(ErrorCode::UnsupportedTokenMintError.into());
    }  

    ai_dex.initialize_reward(
        reward_index as usize,
        ctx.accounts.reward_mint.key(),
        ctx.accounts.reward_vault.key(),
    )?;

    emit!(RewardInitializedEvent {
        reward_index,
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        reward_authority: ctx.accounts.reward_authority.key(),
        funder: ctx.accounts.funder.key(),
        reward_mint: ctx.accounts.reward_mint.key(),
        reward_vault: ctx.accounts.reward_vault.key(),
    });
    
    Ok(())
}
