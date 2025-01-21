// initialize_pool_step_1.rs

use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use crate::{
    errors::ErrorCode,
    math::FULL_RANGE_ONLY_TICK_SPACING_THRESHOLD,
    state::*,
    util::is_supported_token_mint,
};

#[event]
pub struct PoolInitializedBasicEvent {
    pub ai_dex_pool: Pubkey,
    pub ai_dex_config: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub funder: Pubkey,
    pub tick_spacing: u16,
    pub initial_sqrt_price: u128,
    pub default_fee_rate: u16,
    pub fee_tier: Pubkey,
    pub current_tick: i32,
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,
    pub is_temporary_pool: bool,
    pub is_oracle_pool: bool,
    pub oracle_account: Pubkey,
    pub price_update: Pubkey,
    pub price_feed_id: String,
}

/// The `InitializePoolStep1` struct defines the accounts required for the first step of pool initialization.
#[derive(Accounts)]
#[instruction(tick_spacing: u16, is_oracle_pool: bool)]
pub struct InitializePoolStep1<'info> {
    pub ai_dex_config: Box<Account<'info, AiDexConfig>>,

    pub token_mint_a: Box<InterfaceAccount<'info, Mint>>,
    pub token_mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub funder: Signer<'info>,

    #[account(
        init,
        seeds = [
            b"ai_dex".as_ref(),
            ai_dex_config.key().as_ref(),
            token_mint_a.key().as_ref(),
            token_mint_b.key().as_ref(),
            tick_spacing.to_le_bytes().as_ref()
        ],
        bump,
        payer = funder,
        space = AiDexPool::LEN,
    )]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(has_one = ai_dex_config, constraint = fee_tier.tick_spacing == tick_spacing)]
    pub fee_tier: Box<Account<'info, FeeTier>>,

    /// Optional Oracle Account: Only required for Oracle Pools
    #[account(
        init,
        seeds = [
            b"oracle".as_ref(),
            ai_dex_pool.key().as_ref(),
        ],
        bump,
        payer = funder,
        space = OracleAccount::LEN,
        constraint = is_oracle_pool, // Only initialize if pool is oracle
    )]
    pub oracle_account: Option<Account<'info, OracleAccount>>,

    /// Oracle Price Update Account read as AccountInfo
    /// This account can be either a `PriceUpdateV2` from Pyth or a `MockPriceUpdate` from your program
    pub price_update: Option<AccountInfo<'info>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// The `initialize_pool_step_1_handler` function performs the first step of pool initialization.
/// It initializes the AiDex pool and, if applicable, sets up the oracle account.
pub fn initialize_pool_step_1_handler(
    ctx: Context<InitializePoolStep1>,
    tick_spacing: u16,
    is_oracle_pool: bool,
    is_temporary_pool: bool,
    initial_sqrt_price: Option<u128>,  // Required for Classic and Temporary Pools
    price_feed_id: Option<String>,     // Required for Oracle Pools
    maximum_age: Option<u64>,          // Required for Oracle Pools
) -> Result<()> {
    let ai_dex_config = &ctx.accounts.ai_dex_config;
    let mut ai_dex_pool = ctx.accounts.ai_dex_pool.load_init()?;

    let token_mint_a = ctx.accounts.token_mint_a.key();
    let token_mint_b = ctx.accounts.token_mint_b.key();
    let default_fee_rate = ctx.accounts.fee_tier.default_fee_rate;

    // Validate token mints
    if !is_supported_token_mint(&ctx.accounts.token_mint_a)? {
        return Err(ErrorCode::UnsupportedTokenMintError.into());
    }

    if !is_supported_token_mint(&ctx.accounts.token_mint_b)? {
        return Err(ErrorCode::UnsupportedTokenMintError.into());
    }

    // For Oracle pools, ensure tick_spacing meets the threshold
    if is_oracle_pool && tick_spacing < FULL_RANGE_ONLY_TICK_SPACING_THRESHOLD {
        return Err(ErrorCode::InvalidOraclePoolTickSpacing.into());
    }

    // Check if the funder is the config account for temporary pools
    if ctx.accounts.ai_dex_config.config_authority.key() != ctx.accounts.funder.key() && is_temporary_pool {
        return Err(ErrorCode::InvalidTemporaryPoolFunderError.into());
    }

    // Determine initial sqrt price
    let calculated_initial_sqrt_price = if is_oracle_pool {
        // Oracle Pool: Calculate initial sqrt price from price feed
        let oracle_account = ctx
            .accounts
            .oracle_account
            .as_mut()
            .ok_or(ErrorCode::MissingOracleAccount)?;
        
        let price_feed_id = price_feed_id.clone().ok_or(ErrorCode::MissingOraclePriceFeedId)?;
        let maximum_age = maximum_age.ok_or(ErrorCode::MissingMaxAge)?;

        oracle_account.initialize(
            price_feed_id.clone(),
            maximum_age, // Maximum age in seconds
            token_mint_a,
            token_mint_b,
        )?;
        ai_dex_pool.initialize_oracle(oracle_account.key())?;

        let price_update_account_info = ctx
            .accounts
            .price_update
            .as_ref()
            .ok_or(ErrorCode::MissingPriceUpdate)?;

        oracle_account.get_new_sqrt_price(
            &price_update_account_info,
            ctx.accounts.token_mint_a.decimals,
            ctx.accounts.token_mint_b.decimals,
        )?
    } else {
        // Classic or Temporary Pool: Use provided initial sqrt price
        initial_sqrt_price.ok_or(ErrorCode::MissingInitialSqrtPrice)?
    };

    // Initialize Part 1
    ai_dex_pool.initialize_part1(
        ai_dex_config,
        ctx.bumps.ai_dex_pool,
        tick_spacing,
        calculated_initial_sqrt_price,
        default_fee_rate,
        token_mint_a,
        token_mint_b,
        is_temporary_pool,
        is_oracle_pool,
    )?;

    emit!(PoolInitializedBasicEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        ai_dex_config: ai_dex_config.key(),
        token_mint_a,
        token_mint_b,
        funder: ctx.accounts.funder.key(),
        tick_spacing,
        initial_sqrt_price: calculated_initial_sqrt_price,
        default_fee_rate,
        fee_tier: ctx.accounts.fee_tier.key(),
        current_tick: ai_dex_pool.tick_current_index,
        protocol_fee_owed_a: ai_dex_pool.protocol_fee_owed_a,
        protocol_fee_owed_b: ai_dex_pool.protocol_fee_owed_b,
        is_temporary_pool,
        is_oracle_pool,
        oracle_account: ctx.accounts.oracle_account.as_ref().map(|a| a.key()).unwrap_or_default(),
        price_update: ctx.accounts.price_update.as_ref().map(|a| a.key()).unwrap_or_default(),
        price_feed_id: price_feed_id.unwrap_or_default(),
    });

    Ok(())
}
