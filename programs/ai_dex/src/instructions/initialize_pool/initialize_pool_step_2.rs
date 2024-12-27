// initialize_pool_step_2.rs

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface, Mint};
use crate::{
    errors::ErrorCode,
    state::*,
};

#[event]
pub struct PoolInitializedFinalEvent {
    pub ai_dex_pool: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
    pub fee_growth_global_a: u128,
    pub fee_growth_global_b: u128,
    pub start_timestamp_lp: u64,
    pub end_timestamp_lp: u64,
    pub start_timestamp_swap: u64,
    pub end_timestamp_swap: u64,
    pub tick_spacing: u16,
}

/// The `InitializePoolStep2` struct defines the accounts required for the second step of pool initialization.
#[derive(Accounts)]
#[instruction(tick_spacing: u16)]
pub struct InitializePoolStep2<'info> {
    #[account(mut)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    pub token_mint_a: Box<InterfaceAccount<'info, Mint>>,
    pub token_mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub funder: Signer<'info>,

    #[account(
        init,
        payer = funder,
        token::token_program = token_program_a,
        token::mint = token_mint_a,
        token::authority = ai_dex_pool,
        seeds = [
            b"token_vault_a",
            ai_dex_pool.key().as_ref(),
            tick_spacing.to_string().as_bytes(),
        ],
        bump,
    )]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = funder,
        token::token_program = token_program_b,
        token::mint = token_mint_b,
        token::authority = ai_dex_pool,
        seeds = [
            b"token_vault_b",
            ai_dex_pool.key().as_ref(),
            tick_spacing.to_string().as_bytes(),
        ],
        bump,
    )]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        constraint = token_program_a.key() == token_mint_a.to_account_info().owner.clone()
    )]
    pub token_program_a: Interface<'info, TokenInterface>,
    #[account(
        constraint = token_program_b.key() == token_mint_b.to_account_info().owner.clone()
    )]
    pub token_program_b: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// The `initialize_pool_step_2_handler` function performs the second step of pool initialization.
/// It initializes the token vaults and finalizes the pool setup based on the pool's properties.
pub fn initialize_pool_step_2_handler(
    ctx: Context<InitializePoolStep2>,
    tick_spacing: u16,
    start_timestamp_lp: Option<u64>,
    end_timestamp_lp: Option<u64>,
    start_timestamp_swap: Option<u64>,
    end_timestamp_swap: Option<u64>,
) -> Result<()> {
    let ai_dex_pool = &mut ctx.accounts.ai_dex_pool.load_mut()?;

    let pool_token_mint_a = ai_dex_pool.token_mint_a;
    let pool_token_mint_b = ai_dex_pool.token_mint_b;
    if pool_token_mint_a != ctx.accounts.token_mint_a.key() {
        return Err(ErrorCode::InvalidInputTokenMint.into());
    }
    if pool_token_mint_b != ctx.accounts.token_mint_b.key() {
        return Err(ErrorCode::InvalidOutputTokenMint.into());
    }

    if ai_dex_pool.tick_spacing != tick_spacing {
        return Err(ErrorCode::UnsupportedTickSpacing.into());
    }

    // Check if the token vaults have already been initialized
    if ai_dex_pool.token_vault_a != Pubkey::default() || ai_dex_pool.token_vault_b != Pubkey::default() {
        return Err(ErrorCode::VaultAlreadyInitialized.into());
    }

    // Retrieve pool properties
    let is_oracle_pool = ai_dex_pool.is_oracle_pool;
    let is_temporary_pool = ai_dex_pool.is_temporary_pool;

    // Match on the combination of pool properties
    match (is_oracle_pool, is_temporary_pool) {
        (false, false) | (true, false) => {
            // Classic Pool
            // Initialize token vaults
            ai_dex_pool.initialize_part2(
                ctx.accounts.token_vault_a.key(),
                ctx.accounts.token_vault_b.key(),
            )?;
        },
        (false, true) | (true, true) => {
            // Temporary Pool
            // Ensure timestamps are provided
            let start_lp= start_timestamp_lp.ok_or(ErrorCode::MissingTimestamps)?;
            let end_lp = end_timestamp_lp.ok_or(ErrorCode::MissingTimestamps)?;
            let start_swap = start_timestamp_swap.ok_or(ErrorCode::MissingTimestamps)?;
            let end_swap = end_timestamp_swap.ok_or(ErrorCode::MissingTimestamps)?;

            // Initialize token vaults with temporary parameters
            ai_dex_pool.initialize_temp_part2(
                ctx.accounts.token_vault_a.key(),
                ctx.accounts.token_vault_b.key(),
                start_lp,
                end_lp,
                start_swap,
                end_swap,
            )?;

        },
    }

    // Emit PoolInitializedFinalEvent
    emit!(PoolInitializedFinalEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        token_vault_a: ctx.accounts.token_vault_a.key(),
        token_vault_b: ctx.accounts.token_vault_b.key(),
        token_program_a: ctx.accounts.token_program_a.key(),
        token_program_b: ctx.accounts.token_program_b.key(),
        fee_growth_global_a: ai_dex_pool.fee_growth_global_a,
        fee_growth_global_b: ai_dex_pool.fee_growth_global_b,
        start_timestamp_lp: start_timestamp_lp.unwrap_or(0),
        end_timestamp_lp: end_timestamp_lp.unwrap_or(0),
        start_timestamp_swap: start_timestamp_swap.unwrap_or(0),
        end_timestamp_swap: end_timestamp_swap.unwrap_or(0),
        tick_spacing,
    });

    Ok(())
}