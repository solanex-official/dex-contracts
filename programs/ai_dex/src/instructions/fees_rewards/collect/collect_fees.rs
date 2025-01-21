use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::memo::Memo;

use crate::util::{parse_remaining_accounts, AccountsType, RemainingAccountsInfo};
use crate::{
    constants::transfer_memo,
    state::*,
    util::{transfer_from_vault_to_owner, verify_position_authority},
    errors::ErrorCode,
};

#[event]
pub struct FeesCollectedEvent {
    pub ai_dex_pool: Pubkey,
    pub position_authority: Pubkey,
    pub position: Pubkey,
    pub position_token_account: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_owner_account_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_owner_account_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub fee_owed_a: u64,
    pub fee_owed_b: u64,
}

#[derive(Accounts)]
pub struct CollectFees<'info> {
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    pub position_authority: Signer<'info>,

    #[account(mut, has_one = ai_dex_pool)]
    pub position: Box<Account<'info, Position>>,
    #[account(
        constraint = position_token_account.mint == position.position_mint,
        constraint = position_token_account.amount == 1
    )]
    pub position_token_account: Box<Account<'info, token::TokenAccount>>,

    // #[account(address = ai_dex_pool.token_mint_a)]
    #[account(mut)]
    pub token_mint_a: InterfaceAccount<'info, Mint>,
    // #[account(address = ai_dex_pool.token_mint_b)]
    #[account(mut)]
    pub token_mint_b: InterfaceAccount<'info, Mint>,

    // #[account(mut, constraint = token_owner_account_a.mint == ai_dex_pool.token_mint_a)]
    #[account(mut)]
    pub token_owner_account_a: Box<InterfaceAccount<'info, TokenAccount>>,
    // #[account(mut, address = ai_dex_pool.token_vault_a)]
    #[account(mut)]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    // #[account(mut, constraint = token_owner_account_b.mint == ai_dex_pool.token_mint_b)]
    #[account(mut)]
    pub token_owner_account_b: Box<InterfaceAccount<'info, TokenAccount>>,
    // #[account(mut, address = ai_dex_pool.token_vault_b)]
    #[account(mut)]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(constraint = token_program_a.key() == token_mint_a.to_account_info().owner.clone())]
    pub token_program_a: Interface<'info, TokenInterface>,
    #[account(constraint = token_program_b.key() == token_mint_b.to_account_info().owner.clone())]
    pub token_program_b: Interface<'info, TokenInterface>,
    pub memo_program: Program<'info, Memo>,

}

/// Handles the collection of fees for a given position.
///
/// This function verifies the authority of the position, processes any remaining accounts,
/// and transfers the owed fees from the vault to the owner's account.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for the fee collection.
/// * `remaining_accounts_info` - Optional information about remaining accounts.
///
/// # Returns
///
/// * `Result<()>` - Returns an Ok result if the fee collection is successful, otherwise returns an error.
///
/// # Errors
///
/// This function will return an error if:
/// * The position authority verification fails.
/// * Parsing the remaining accounts fails.
/// * Transferring fees from the vault to the owner fails.
pub fn collect_fees_handler<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CollectFees<'info>>,
    remaining_accounts_info: Option<RemainingAccountsInfo>,
) -> Result<()> {
    verify_position_authority(
        &ctx.accounts.position_token_account,
        &ctx.accounts.position_authority,
    )?;

    let ai_dex_pool = ctx.accounts.ai_dex_pool.load()?;

    // Validate token mints against the pool's expected mints.
    if ctx.accounts.token_mint_a.key() != ai_dex_pool.token_mint_a {
        return Err(ErrorCode::InvalidInputTokenMint.into());
    }
    if ctx.accounts.token_mint_b.key() != ai_dex_pool.token_mint_b {
        return Err(ErrorCode::InvalidOutputTokenMint.into());
    }
    if ctx.accounts.token_owner_account_a.mint != ai_dex_pool.token_mint_a {
        return Err(ErrorCode::InvalidTokenOwner.into());
    }
    if ctx.accounts.token_vault_a.key() != ai_dex_pool.token_vault_a {
        return Err(ErrorCode::InvalidVault.into());
    }
    if ctx.accounts.token_owner_account_b.mint != ai_dex_pool.token_mint_b {
        return Err(ErrorCode::InvalidTokenOwner.into());
    }
    if ctx.accounts.token_vault_b.key() != ai_dex_pool.token_vault_b {
        return Err(ErrorCode::InvalidVault.into());
    }

    // Process remaining accounts
    let remaining_accounts = parse_remaining_accounts(
        &ctx.remaining_accounts,
        &remaining_accounts_info,
        &[
            AccountsType::TransferHookA,
            AccountsType::TransferHookB,
        ],
    )?;

    let position = &mut ctx.accounts.position;

    // Store the fees owed to use as transfer amounts.
    let fee_owed_a = position.fee_owed_a;
    let fee_owed_b = position.fee_owed_b;

    // Reset fees owed on the position before transferring.
    position.reset_fees_owed();

    // Conditionally transfer owed fees for Token A if non-zero.
    if fee_owed_a > 0 {
        transfer_from_vault_to_owner(
            &ctx.accounts.ai_dex_pool,
            &ctx.accounts.token_mint_a,
            &ctx.accounts.token_vault_a,
            &ctx.accounts.token_owner_account_a,
            &ctx.accounts.token_program_a,
            &ctx.accounts.memo_program,
            &remaining_accounts.transfer_hook_a,
            fee_owed_a,
            transfer_memo::TRANSFER_MEMO_COLLECT_FEES.as_bytes(),
        )?;
    }

    // Conditionally transfer owed fees for Token B if non-zero.
    if fee_owed_b > 0 {
        transfer_from_vault_to_owner(
            &ctx.accounts.ai_dex_pool,
            &ctx.accounts.token_mint_b,
            &ctx.accounts.token_vault_b,
            &ctx.accounts.token_owner_account_b,
            &ctx.accounts.token_program_b,
            &ctx.accounts.memo_program,
            &remaining_accounts.transfer_hook_b,
            fee_owed_b,
            transfer_memo::TRANSFER_MEMO_COLLECT_FEES.as_bytes(),
        )?;
    }

    emit!(FeesCollectedEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        position_authority: ctx.accounts.position_authority.key(),
        position: ctx.accounts.position.key(),
        position_token_account: ctx.accounts.position_token_account.key(),
        token_mint_a: ctx.accounts.token_mint_a.key(),
        token_mint_b: ctx.accounts.token_mint_b.key(),
        token_owner_account_a: ctx.accounts.token_owner_account_a.key(),
        token_vault_a: ctx.accounts.token_vault_a.key(),
        token_owner_account_b: ctx.accounts.token_owner_account_b.key(),
        token_vault_b: ctx.accounts.token_vault_b.key(),
        fee_owed_a,
        fee_owed_b,
    });

    Ok(())
}
