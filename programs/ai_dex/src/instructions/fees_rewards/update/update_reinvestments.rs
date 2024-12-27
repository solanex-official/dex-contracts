use anchor_lang::prelude::*;
use anchor_spl::token;
use crate::{
    errors::ErrorCode,
    math::convert_to_liquidity_delta,
    orchestrator::liquidity_orchestrator::{
        calculate_modify_liquidity,
        sync_modify_liquidity_values
    },
    state::*,
    util::{
        calculate_liquidity_from_amounts,
        calculate_reinvestment_amounts,
        calculate_reinvestment_fees,
        to_timestamp_u64
    }, UpdateTicksEvent,
};

#[event]
pub struct ReinvestFeesEvent {
    pub ai_dex_pool: Pubkey,
    pub position: Pubkey,
    pub reinvestments_authority: Pubkey,
    pub reinvested_amount_a: u64,
    pub reinvested_amount_b: u64,
    pub liquidity_delta_added: u128,
    pub protocol_fee_added_a: u64,
    pub protocol_fee_added_b: u64,
}

#[derive(Accounts)]
pub struct ReinvestFees<'info> {
    #[account(mut)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,
    
    #[account(address = reinvestments_account.reinvestments_authority)]
    pub reinvestments_authority: Signer<'info>,

    #[account(mut, has_one = ai_dex_pool)]
    pub position: Account<'info, Position>,
    #[account(
        constraint = position_token_account.mint == position.position_mint,
        constraint = position_token_account.amount == 1
    )]
    pub position_token_account: Box<Account<'info, token::TokenAccount>>,

    #[account(mut, has_one = ai_dex_pool)]
    pub tick_array_lower: AccountLoader<'info, TickArray>,
    #[account(mut, has_one = ai_dex_pool)]
    pub tick_array_upper: AccountLoader<'info, TickArray>,

    pub reinvestments_account: Account<'info, AiDexReinvestments>,
}

pub fn reinvest_fees_handler(
    ctx: Context<ReinvestFees>,
) -> Result<()> {
    if !ctx.accounts.position.is_reinvestment_on {
        return Err(ErrorCode::ReinvestmentNotEnabled.into());
    }

    let position = &mut ctx.accounts.position;
    let mut ai_dex_pool = ctx.accounts.ai_dex_pool.load_mut()?;
    
    // Calculate amounts based on current tick position
    let (amount_a, amount_b) = calculate_reinvestment_amounts(
        position.fee_owed_a,
        position.fee_owed_b,
        ai_dex_pool.sqrt_price,
        ai_dex_pool.tick_current_index,
        position.tick_lower_index,
        position.tick_upper_index,
    )?;

    if amount_a == 0 && amount_b == 0 {
        return Ok(());
    }
    
    // Calculate protocol fees
    let (protocol_fee_a, protocol_fee_b, reinvest_amount_a, reinvest_amount_b) = 
        calculate_reinvestment_fees(
            amount_a,
            amount_b,
            ctx.accounts.reinvestments_account.default_reinvestment_fee_rate,
        );
    
    // Update protocol fees in pool
    ai_dex_pool.add_protocol_fees_owed(protocol_fee_a, protocol_fee_b);

    // Calculate liquidity based on the amounts
    let liquidity_delta = convert_to_liquidity_delta(
        calculate_liquidity_from_amounts(
            ai_dex_pool.tick_current_index,
            ai_dex_pool.sqrt_price,
            position.tick_lower_index,
            position.tick_upper_index,
            reinvest_amount_a,
            reinvest_amount_b,
        )?,
        true,
    )?;

    let timestamp = to_timestamp_u64(Clock::get()?.unix_timestamp)?;

    let update = calculate_modify_liquidity(
        &ai_dex_pool,
        position,
        &ctx.accounts.tick_array_lower,
        &ctx.accounts.tick_array_upper,
        liquidity_delta,
        timestamp,
    )?;

    sync_modify_liquidity_values(
        &mut ai_dex_pool,
        position,
        &ctx.accounts.tick_array_lower,
        &ctx.accounts.tick_array_upper,
        update,
        timestamp,
    )?;

    // Subtract the reinvested amounts from fees owed
    position.subtract_fees_owed(reinvest_amount_a, reinvest_amount_b);

    emit!(UpdateTicksEvent {
        tick_lower_index: position.tick_lower_index,
        tick_lower_update: update.tick_lower_update,
        tick_upper_index: position.tick_upper_index,
        tick_upper_update: update.tick_upper_update,
        tick_array_lower: ctx.accounts.tick_array_lower.key(),
        tick_array_upper: ctx.accounts.tick_array_upper.key(),
    });
    
    emit!(ReinvestFeesEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        position: position.key(),
        reinvestments_authority: ctx.accounts.reinvestments_authority.key(),
        reinvested_amount_a: reinvest_amount_a,
        reinvested_amount_b: reinvest_amount_b,
        liquidity_delta_added: liquidity_delta.abs() as u128,
        protocol_fee_added_a: protocol_fee_a,
        protocol_fee_added_b: protocol_fee_b,
    });

    Ok(())
}