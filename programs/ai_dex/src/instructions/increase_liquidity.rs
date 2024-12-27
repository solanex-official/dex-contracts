use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::memo::Memo;

use crate::errors::ErrorCode;
use crate::orchestrator::liquidity_orchestrator::{
    calculate_liquidity_token_deltas, calculate_modify_liquidity, sync_modify_liquidity_values,
};
use crate::math::convert_to_liquidity_delta;
use crate::state::*;
use crate::util::{calculate_transfer_fee_included_amount, parse_remaining_accounts, AccountsType, RemainingAccountsInfo};
use crate::util::{to_timestamp_u64, transfer_from_owner_to_vault, verify_position_authority};

#[event]
pub struct IncreaseLiquidityEvent {
    pub liquidity_amount: u128,
    pub token_max_a: u64,
    pub token_max_b: u64,
    pub position_authority: Pubkey,
    pub position: Pubkey,
    pub ai_dex_pool: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub token_owner_account_a: Pubkey,
    pub token_owner_account_b: Pubkey,
    pub delta_a: u64,
    pub delta_b: u64,
    pub transfer_fee_included_delta_a: u64,
    pub transfer_fee_included_delta_b: u64,
    pub sqrt_price: u128,
    pub new_liquidity_value: u128,
    pub update_position: PositionUpdate,
    pub referral_code: String,
    pub timestamp: u64,
}

#[event]
pub struct UpdateTicksEvent {
    pub tick_lower_index: i32,
    pub tick_lower_update: TickUpdate,
    pub tick_upper_index: i32,
    pub tick_upper_update: TickUpdate,
    pub tick_array_lower: Pubkey,
    pub tick_array_upper: Pubkey,
}

#[derive(Accounts)]
pub struct ModifyLiquidity<'info> {
    #[account(mut)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(
        constraint = token_program_a.key() == token_mint_a.to_account_info().owner.clone()
    )]
    pub token_program_a: Interface<'info, TokenInterface>,
    #[account(
        constraint = token_program_b.key() == token_mint_b.to_account_info().owner.clone()
    )]
    pub token_program_b: Interface<'info, TokenInterface>,

    pub memo_program: Program<'info, Memo>,

    pub position_authority: Signer<'info>,

    #[account(mut, has_one = ai_dex_pool)]
    pub position: Account<'info, Position>,
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
    // #[account(mut, constraint = token_owner_account_b.mint == ai_dex_pool.token_mint_b)]
    #[account(mut)]
    pub token_owner_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    // #[account(mut, constraint = token_vault_a.key() == ai_dex_pool.token_vault_a)]
    #[account(mut)]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,
    // #[account(mut, constraint = token_vault_b.key() == ai_dex_pool.token_vault_b)]
    #[account(mut)]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, has_one = ai_dex_pool)]
    pub tick_array_lower: AccountLoader<'info, TickArray>,
    #[account(mut, has_one = ai_dex_pool)]
    pub tick_array_upper: AccountLoader<'info, TickArray>,

    #[account(
        mut,
        constraint = oracle_account.mint_a == token_mint_a.key() && oracle_account.mint_b == token_mint_b.key()
    )]
    pub oracle_account: Option<Account<'info, OracleAccount>>,

    /// Oracle Price Update Account: Can be either a real PriceUpdateV2 or a MockPriceUpdate
    pub price_update: Option<AccountInfo<'info>>,

}

/// Handles the increase of liquidity in the protocol.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts and programs required for the operation.
/// * `liquidity_amount` - The amount of liquidity to be added.
/// * `token_max_a` - The maximum amount of token A that can be transferred.
/// * `token_max_b` - The maximum amount of token B that can be transferred.
/// * `remaining_accounts_info` - Optional information about remaining accounts.
///
/// # Returns
///
/// * `Result<()>` - Returns an Ok result if the operation is successful, otherwise returns an error.
///
/// # Errors
///
/// * `ErrorCode::ZeroLiquidityError` - If the liquidity amount is zero.
/// * `ErrorCode::TokenLimitExceededError` - If the transfer amount exceeds the specified token limits.
pub fn increase_liquidity_handler<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, ModifyLiquidity<'info>>,
    liquidity_amount: u128,
    token_max_a: u64,
    token_max_b: u64,
    remaining_accounts_info: Option<RemainingAccountsInfo>,
    referral_code: Option<String>,
) -> Result<()> {
    verify_position_authority(
        &ctx.accounts.position_token_account,
        &ctx.accounts.position_authority,
    )?;

    if liquidity_amount == 0 {
        return Err(ErrorCode::ZeroLiquidityError.into());
    }

    // Load AiDexPool as mut from the AccountLoader
    let mut ai_dex_pool_mut = ctx.accounts.ai_dex_pool.load_mut()?;  // Mutable borrow

    // Implementing the commented checks
    if ctx.accounts.token_mint_a.key() != ai_dex_pool_mut.token_mint_a {
        return Err(ErrorCode::InvalidInputTokenMint.into());
    }

    if ctx.accounts.token_mint_b.key() != ai_dex_pool_mut.token_mint_b {
        return Err(ErrorCode::InvalidOutputTokenMint.into());
    }

    if ctx.accounts.token_owner_account_a.mint != ai_dex_pool_mut.token_mint_a {
        return Err(ErrorCode::InvalidTokenOwner.into());
    }

    if ctx.accounts.token_owner_account_b.mint != ai_dex_pool_mut.token_mint_b {
        return Err(ErrorCode::InvalidTokenOwner.into());
    }

    if ctx.accounts.token_vault_a.key() != ai_dex_pool_mut.token_vault_a {
        return Err(ErrorCode::InvalidVault.into());
    }

    if ctx.accounts.token_vault_b.key() != ai_dex_pool_mut.token_vault_b {
        return Err(ErrorCode::InvalidVault.into());
    }

    if ai_dex_pool_mut.is_oracle_pool {
        let oracle_account = ctx
            .accounts
            .oracle_account
            .as_mut()
            .ok_or(ErrorCode::MissingOracleAccount)?;
        let price_update_account_info = ctx
            .accounts
            .price_update
            .as_ref()
            .ok_or(ErrorCode::MissingPriceUpdate)?;

        oracle_account.update_sqrt_price(
            &mut *ai_dex_pool_mut,
            price_update_account_info,
            ctx.accounts.token_mint_a.decimals,
            ctx.accounts.token_mint_b.decimals,
        )?;
    }

    let timestamp = to_timestamp_u64(Clock::get()?.unix_timestamp)?;

    let remaining_accounts = parse_remaining_accounts(
        &ctx.remaining_accounts,
        &remaining_accounts_info,
        &[AccountsType::TransferHookA, AccountsType::TransferHookB],
    )?;

    let liquidity_delta = convert_to_liquidity_delta(liquidity_amount, true)?;

    let update = calculate_modify_liquidity(
        &ai_dex_pool_mut,
        &ctx.accounts.position,
        &ctx.accounts.tick_array_lower,
        &ctx.accounts.tick_array_upper,
        liquidity_delta,
        timestamp,
    )?;

    sync_modify_liquidity_values(
        &mut ai_dex_pool_mut,
        &mut ctx.accounts.position,
        &ctx.accounts.tick_array_lower,
        &ctx.accounts.tick_array_upper,
        update,
        timestamp,
    )?;

    let (delta_a, delta_b) = calculate_liquidity_token_deltas(
        ai_dex_pool_mut.tick_current_index,
        ai_dex_pool_mut.sqrt_price,
        &ctx.accounts.position,
        liquidity_delta,
    )?;

    let transfer_fee_included_delta_a = calculate_transfer_fee_included_amount(
        &ctx.accounts.token_mint_a,
        delta_a,
    )?;
    let transfer_fee_included_delta_b = calculate_transfer_fee_included_amount(
        &ctx.accounts.token_mint_b,
        delta_b,
    )?;

    // token_max_a and token_max_b should be applied to the transfer fee included amount
    if transfer_fee_included_delta_a.amount > token_max_a {
        return Err(ErrorCode::TokenLimitExceededError.into());
    }
    if transfer_fee_included_delta_b.amount > token_max_b {
        return Err(ErrorCode::TokenLimitExceededError.into());
    }

    transfer_from_owner_to_vault(
        &ctx.accounts.position_authority,
        &ctx.accounts.token_mint_a,
        &ctx.accounts.token_owner_account_a,
        &ctx.accounts.token_vault_a,
        &ctx.accounts.token_program_a,
        &ctx.accounts.memo_program,
        &remaining_accounts.transfer_hook_a,
        transfer_fee_included_delta_a.amount,
    )?;

    transfer_from_owner_to_vault(
        &ctx.accounts.position_authority,
        &ctx.accounts.token_mint_b,
        &ctx.accounts.token_owner_account_b,
        &ctx.accounts.token_vault_b,
        &ctx.accounts.token_program_b,
        &ctx.accounts.memo_program,
        &remaining_accounts.transfer_hook_b,
        transfer_fee_included_delta_b.amount,
    )?;

    emit!(UpdateTicksEvent {
        tick_lower_index: ctx.accounts.position.tick_lower_index,
        tick_lower_update: update.tick_lower_update,
        tick_upper_index: ctx.accounts.position.tick_upper_index,
        tick_upper_update: update.tick_upper_update,
        tick_array_lower: ctx.accounts.tick_array_lower.key(),
        tick_array_upper: ctx.accounts.tick_array_upper.key(),
    });

    emit!(IncreaseLiquidityEvent {
        liquidity_amount,
        token_max_a,
        token_max_b,
        position_authority: ctx.accounts.position_authority.key(),
        position: ctx.accounts.position.key(),
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        token_mint_a: ctx.accounts.token_mint_a.key(),
        token_mint_b: ctx.accounts.token_mint_b.key(),
        token_vault_a: ctx.accounts.token_vault_a.key(),
        token_vault_b: ctx.accounts.token_vault_b.key(),
        token_owner_account_a: ctx.accounts.token_owner_account_a.key(),
        token_owner_account_b: ctx.accounts.token_owner_account_b.key(),
        delta_a,
        delta_b,
        transfer_fee_included_delta_a: transfer_fee_included_delta_a.amount,
        transfer_fee_included_delta_b: transfer_fee_included_delta_b.amount,
        sqrt_price: ai_dex_pool_mut.sqrt_price,
        new_liquidity_value: ai_dex_pool_mut.liquidity,
        update_position: update.position_update,
        referral_code: referral_code.unwrap_or_default(),
        timestamp,
    });

    Ok(())
}
