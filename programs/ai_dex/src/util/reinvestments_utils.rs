use anchor_lang::prelude::*;
use crate::{
    errors::ErrorCode,
    math::{get_amount_delta_a, get_amount_delta_b, sqrt_price_from_tick_index, REINVESTMENT_PROTOCOL_FEE_RATE_MUL_VALUE},
};

pub fn calculate_reinvestment_amounts(
    fee_owed_a: u64,
    fee_owed_b: u64,
    sqrt_price: u128,
    current_tick_index: i32,
    position_tick_lower: i32,
    position_tick_upper: i32,
) -> Result<(u64, u64)> {
    // First determine which tokens we need based on current tick position
    match current_tick_index {
        // Current tick below position - only need token A
        _ if current_tick_index < position_tick_lower => {
            Ok((fee_owed_a, 0))
        },
        // Current tick above position - only need token B
        _ if current_tick_index >= position_tick_upper => {
            Ok((0, fee_owed_b))
        },
        // Current tick inside position - need both tokens proportional to current sqrt price
        _ => {
            // Calculate the ideal ratio at current price
            let sqrt_price_lower = sqrt_price_from_tick_index(position_tick_lower);
            let sqrt_price_upper = sqrt_price_from_tick_index(position_tick_upper);

            // Get amounts needed for 1 unit of liquidity at current price
            let amount_a_per_liquidity = get_amount_delta_a(sqrt_price, sqrt_price_upper, 1, true)?;
            let amount_b_per_liquidity = get_amount_delta_b(sqrt_price_lower, sqrt_price, 1, true)?;

            // Convert both token amounts to a common unit using the current price
            let value_in_a = fee_owed_a as u128 + ((fee_owed_b as u128 * (1u128 << 64)) / sqrt_price);
            
            // Calculate the optimal amounts maintaining price ratio
            let total_value_in_ticks = amount_a_per_liquidity as u128 + 
                ((amount_b_per_liquidity as u128 * (1u128 << 64)) / sqrt_price);
            
            let optimal_liquidity = value_in_a
                .checked_mul(1u128 << 64)
                .ok_or(ErrorCode::LiquidityOverflowError)?
                .checked_div(total_value_in_ticks)
                .ok_or(ErrorCode::LiquidityOverflowError)?;

            // Calculate final amounts maintaining the price ratio
            let amount_a = ((optimal_liquidity * amount_a_per_liquidity as u128) >> 64) as u64;
            let amount_b = ((optimal_liquidity * amount_b_per_liquidity as u128) >> 64) as u64;

            // Ensure we don't exceed available fees
            let final_amount_a = std::cmp::min(amount_a, fee_owed_a);
            let final_amount_b = std::cmp::min(amount_b, fee_owed_b);

            Ok((final_amount_a, final_amount_b))
        }
    }
}

pub fn calculate_liquidity_from_amounts(
    current_tick_index: i32,
    sqrt_price: u128,
    tick_lower_index: i32,
    tick_upper_index: i32,
    amount_a: u64,
    amount_b: u64,
) -> Result<u128> {
    if tick_upper_index < tick_lower_index {
        return Err(ErrorCode::InvalidTickArraySequenceError.into());
    }

    // Early return if both amounts are zero
    if amount_a == 0 && amount_b == 0 {
        return Ok(0);
    }

    let sqrt_price_lower = sqrt_price_from_tick_index(tick_lower_index);
    let sqrt_price_upper = sqrt_price_from_tick_index(tick_upper_index);

    let liquidity = if current_tick_index >= tick_upper_index {
        // Only token B calculation
        if amount_b == 0 {
            // If no token B amount provided, liquidity is zero
            return Ok(0);
        }
        est_liquidity_for_token_b(sqrt_price_upper, sqrt_price_lower, amount_b)?
    } else if current_tick_index < tick_lower_index {
        // Only token A calculation
        if amount_a == 0 {
            // If no token A amount provided, liquidity is zero
            return Ok(0);
        }
        est_liquidity_for_token_a(sqrt_price_lower, sqrt_price_upper, amount_a)?
    } else {
        // Both token calculations
        if amount_a == 0 || amount_b == 0 {
            // If either amount is zero in the overlapping range, liquidity is zero
            return Ok(0);
        }
        let liquidity_a = est_liquidity_for_token_a(sqrt_price, sqrt_price_upper, amount_a)?;
        let liquidity_b = est_liquidity_for_token_b(sqrt_price, sqrt_price_lower, amount_b)?;
        std::cmp::min(liquidity_a, liquidity_b)
    };

    Ok(liquidity)
}

fn est_liquidity_for_token_a(
    sqrt_price1: u128,
    sqrt_price2: u128,
    token_amount: u64,
) -> Result<u128> {
    let lower_sqrt_price = std::cmp::min(sqrt_price1, sqrt_price2);
    let upper_sqrt_price = std::cmp::max(sqrt_price1, sqrt_price2);

    // First multiplication
    let first_mul = (token_amount as u128)
        .checked_mul(upper_sqrt_price)
        .ok_or(ErrorCode::LiquidityOverflowError)?;

    // Shift down after first multiplication to prevent overflow
    let shifted_first = first_mul >> 64;
    
    // Second multiplication after shifting
    let second_mul = shifted_first
        .checked_mul(lower_sqrt_price)
        .ok_or(ErrorCode::LiquidityOverflowError)?;

    // Denominator calculation
    let denominator = upper_sqrt_price
        .checked_sub(lower_sqrt_price)
        .ok_or(ErrorCode::LiquidityOverflowError)?;
    
    // Final division
    second_mul
        .checked_div(denominator)
        .ok_or(ErrorCode::LiquidityOverflowError.into())
}

fn est_liquidity_for_token_b(
    sqrt_price1: u128,
    sqrt_price2: u128,
    token_amount: u64,
) -> Result<u128> {
    let lower_sqrt_price = std::cmp::min(sqrt_price1, sqrt_price2);
    let upper_sqrt_price = std::cmp::max(sqrt_price1, sqrt_price2);

    let delta = upper_sqrt_price
        .checked_sub(lower_sqrt_price)
        .ok_or(ErrorCode::LiquidityOverflowError)?;

    (token_amount as u128)
        .checked_shl(64)
        .ok_or(ErrorCode::LiquidityOverflowError)?
        .checked_div(delta)
        .ok_or(ErrorCode::LiquidityOverflowError.into())
}

pub fn calculate_reinvestment_fees(
    amount_a: u64,
    amount_b: u64,
    reinvest_fee_rate: u16,
) -> (u64, u64, u64, u64) {
    // Returns (protocol_fee_a, protocol_fee_b, reinvestment_amount_a_after_fees, reinvestment_amount_b_after_fees)
    
    if reinvest_fee_rate == 0 {
        return (0, 0, amount_a, amount_b);
    }

    // Calculate protocol fees proportionally for each token
    let protocol_fee_a = ((amount_a as u128) * (reinvest_fee_rate as u128)
        / REINVESTMENT_PROTOCOL_FEE_RATE_MUL_VALUE)
        .try_into()
        .unwrap_or(0);
        
    let protocol_fee_b = ((amount_b as u128) * (reinvest_fee_rate as u128)
        / REINVESTMENT_PROTOCOL_FEE_RATE_MUL_VALUE)
        .try_into()
        .unwrap_or(0);

    // Subtract protocol fees from reinvestment amounts
    let final_amount_a = amount_a.wrapping_sub(protocol_fee_a);
    let final_amount_b = amount_b.wrapping_sub(protocol_fee_b);

    (protocol_fee_a, protocol_fee_b, final_amount_a, final_amount_b)
}