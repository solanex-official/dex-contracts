use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::Price;
use crate::errors::ErrorCode;

/// Calculates the initial sqrt price from Pyth Oracle price data.
///
/// # Parameters
/// - price_data: The price data obtained from the Pyth Oracle.
/// - decimals_a: The number of decimal places for Token A.
/// - decimals_b: The number of decimal places for Token B.
///
/// # Returns
/// - Result<u128>: The calculated initial sqrt price in Q64.64 fixed-point format.
///
/// # Errors
/// - ErrorCode::InvalidPrice: If the price is non-positive.
/// - ErrorCode::MultiplicationOverflowError: If any multiplication operation overflows.
/// - ErrorCode::DivisionByZeroError: If a division by zero is attempted.
pub fn calculate_initial_sqrt_price(price_data: &Price, decimals_a: u8, decimals_b: u8) -> Result<u128> {
    // Step 1: Validate the price
    if price_data.price <= 0 {
        return Err(ErrorCode::InvalidPrice.into());
    }

    // Step 2: Adjust the exponent based on token decimals
    // exponent_adjustment = price_data.exponent + (decimals_b - decimals_a)
    let exponent_adjustment: i32 = price_data.exponent as i32 + (decimals_b as i32) - (decimals_a as i32);

    // Step 3: Calculate numerator and denominator based on exponent adjustment
    let (numerator, denominator) = if exponent_adjustment >= 0 {
        // If exponent_adjustment is non-negative, scale the price up
        let pow10 = 10u128
            .checked_pow(exponent_adjustment as u32)
            .ok_or(ErrorCode::MultiplicationOverflowError)?;
        let numerator = (price_data.price as u128)
            .checked_mul(pow10)
            .ok_or(ErrorCode::MultiplicationOverflowError)?;
        (numerator, 1u128)
    } else {
        // If exponent_adjustment is negative, scale the price down
        let pow10 = 10u128
            .checked_pow((-exponent_adjustment) as u32)
            .ok_or(ErrorCode::MultiplicationOverflowError)?;
        let denominator = pow10;
        (price_data.price as u128, denominator)
    };

    // Step 4: Compute integer square roots
    let sqrt_numerator = integer_sqrt(numerator);
    let sqrt_denominator = integer_sqrt(denominator);

    // Step 5: Convert to fixed-point format (Q64.64)
    let initial_sqrt_price = compute_sqrt_price_fixed_point(sqrt_numerator, sqrt_denominator)?;

    Ok(initial_sqrt_price)
}

/// Computes the initial sqrt price in Q64.64 fixed-point format.
///
/// # Parameters
/// - sqrt_numerator: The integer square root of the numerator.
/// - sqrt_denominator: The integer square root of the denominator.
///
/// # Returns
/// - Result<u128>: The fixed-point initial sqrt price.
///
/// # Errors
/// - ErrorCode::DivisionByZeroError: If the denominator is zero.
/// - ErrorCode::MultiplicationOverflowError: If shifting the numerator overflows.
fn compute_sqrt_price_fixed_point(sqrt_numerator: u128, sqrt_denominator: u128) -> Result<u128> {
    // Ensure sqrt_denominator is not zero to prevent division by zero
    if sqrt_denominator == 0 {
        return Err(ErrorCode::DivisionByZeroError.into());
    }

    // Check if shifting sqrt_numerator by 64 bits would overflow u128
    // u128 has 128 bits, so sqrt_numerator must be <= 2^(128 - 64) -1 = 2^64 -1 = 18_446_744_073_709_551_615
    if sqrt_numerator > u64::MAX as u128 {
        return Err(ErrorCode::MultiplicationOverflowError.into());
    }

    // Shift sqrt_numerator left by 64 bits to convert to Q64.64 fixed-point format
    let shifted_numerator = sqrt_numerator
        .checked_shl(64)
        .ok_or(ErrorCode::MultiplicationOverflowError)?;

    // Perform division to get the final fixed-point sqrt price
    let initial_sqrt_price = shifted_numerator
        .checked_div(sqrt_denominator)
        .ok_or(ErrorCode::DivisionByZeroError)?;

    Ok(initial_sqrt_price)
}

/// Computes the integer square root of a u128 using the binary search method.
///
/// # Parameters
/// - n: The number to compute the square root of.
///
/// # Returns
/// - u128: The integer square root of n.
fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut low: u128 = 1;
    let mut high: u128 = n;
    let mut mid: u128;
    let mut res: u128 = 0;

    while low <= high {
        mid = low + (high - low) / 2;
        let mid_sq = mid.checked_mul(mid).unwrap_or(n + 1); // Prevent overflow
        if mid_sq == n {
            return mid;
        } else if mid_sq < n {
            res = mid;
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyth_solana_receiver_sdk::price_update::Price;

    /// Helper function to create a Price instance
    fn create_price(price: i64, exponent: i32) -> Price {
        Price {
            price,
            conf: 0, // Confidence interval (not used in calculations here)
            exponent,
            publish_time: 0,
        }
    }

    #[test]
    fn test_price_conversion_back_and_forth() {
        // Use the sqrt price to get back the original price
        let price_data = create_price(7_160_106_530_699, -8);
        let decimals_a = 6u8;
        let decimals_b = 8u8;

        let sqrt_price_x64 = calculate_initial_sqrt_price(&price_data, decimals_a, decimals_b).unwrap();
        msg!("sqrt_price_x64: {}", sqrt_price_x64);

        // Convert sqrt_price_x64 back to price
        let price_reconstructed = sqrt_price_x64_to_price(sqrt_price_x64, decimals_a, decimals_b).unwrap();

        // Original price adjusted for decimals
        let original_price = (price_data.price as f64) * 10f64.powi(price_data.exponent);

        // Allow a small margin due to floating-point arithmetic
        let margin = 0.05;

        assert!(
            (price_reconstructed - original_price).abs() <= margin,
            "Reconstructed price does not match original. Expected: {}, Got: {}",
            original_price,
            price_reconstructed
        );
    }

    fn sqrt_price_x64_to_price(sqrt_price_x64: u128, decimals_a: u8, decimals_b: u8) -> Result<f64> {
        // Function to convert u128 to f64 without losing precision
        fn u128_to_f64(u: u128) -> f64 {
            const TWO64: f64 = 18446744073709551616.0; // 2^64
            let high = (u >> 64) as u64;
            let low = u as u64;
            (high as f64) * TWO64 + (low as f64)
        }

        let sqrt_price = u128_to_f64(sqrt_price_x64) / u128_to_f64(1u128 << 64);
        let price_adj = sqrt_price * sqrt_price;

        // Adjust price for decimals difference
        let exponent_adjustment = (decimals_a as i32) - (decimals_b as i32);
        let price = price_adj * 10f64.powi(exponent_adjustment);

        Ok(price)
    }

    #[test]
    fn test_negative_price() {
        let price_data = create_price(-1_000_000, -8);
        let decimals_a = 6u8;
        let decimals_b = 8u8;

        let result = calculate_initial_sqrt_price(&price_data, decimals_a, decimals_b);

        assert!(result.is_err(), "Expected error for negative price");
    }

    #[test]
    fn test_calculate_initial_sqrt_price_positive_price_positive_exponent() -> Result<()> {
        // Test with positive price and zero exponent adjustment
        let price_data = create_price(100_000_000, -8);
        let decimals_a = 6;
        let decimals_b = 6;

        let result = calculate_initial_sqrt_price(&price_data, decimals_a, decimals_b)?;

        // Expected sqrt_price_x64 is 1 << 64
        assert_eq!(result, 18446744073709551616);

        Ok(())
    }

    #[test]
    fn test_calculate_initial_sqrt_price_zero_price() {
        // Test with zero price, should return InvalidPrice error
        let price_data = create_price(0, -8);
        let decimals_a = 6;
        let decimals_b = 6;

        let result = calculate_initial_sqrt_price(&price_data, decimals_a, decimals_b);

        match result {
            Err(e) => {
                if let anchor_lang::error::Error::AnchorError(anchor_error) = e {
                    assert_eq!(
                        anchor_error.error_code_number,
                        ErrorCode::InvalidPrice as u32 + 6000
                    );
                    assert_eq!(anchor_error.error_name, "InvalidPrice");
                } else {
                    panic!("Expected AnchorError, got {:?}", e);
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn test_calculate_initial_sqrt_price_negative_price() {
        // Test with negative price, should return InvalidPrice error
        let price_data = create_price(-100_000_000, -8);
        let decimals_a = 6;
        let decimals_b = 6;

        let result = calculate_initial_sqrt_price(&price_data, decimals_a, decimals_b);

        match result {
            Err(e) => {
                if let anchor_lang::error::Error::AnchorError(anchor_error) = e {
                    assert_eq!(
                        anchor_error.error_code_number,
                        ErrorCode::InvalidPrice as u32 + 6000
                    );
                    assert_eq!(anchor_error.error_name, "InvalidPrice");
                } else {
                    panic!("Expected AnchorError, got {:?}", e);
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn test_calculate_initial_sqrt_price_positive_price_negative_exponent() -> Result<()> {
        // Test with positive price and negative exponent
        let price_data = create_price(1_000_000_000, -9);
        let decimals_a = 6;
        let decimals_b = 6;

        let result = calculate_initial_sqrt_price(&price_data, decimals_a, decimals_b)?;

        // Expected sqrt_price_x64 is 1 << 64
        assert_eq!(result, 18446744073709551616);

        Ok(())
    }

    #[test]
    fn test_calculate_initial_sqrt_price_exponent_adjustment_positive() -> Result<()> {
        // Test with exponent adjustment positive
        let price_data = create_price(50_000_000, -8);
        let decimals_a = 8;
        let decimals_b = 6; // decimals_b - decimals_a = -2

        let result = calculate_initial_sqrt_price(&price_data, decimals_a, decimals_b)?;

        // Manually calculate expected result
        let exponent_adjustment = -8 + (-2);
        let numerator = price_data.price as u128;
        let denominator = 10u128.pow((-exponent_adjustment) as u32);
        let sqrt_numerator = integer_sqrt(numerator);
        let sqrt_denominator = integer_sqrt(denominator);
        let shifted_numerator = sqrt_numerator << 64;
        let expected_sqrt_price = shifted_numerator / sqrt_denominator;

        assert_eq!(result, expected_sqrt_price);

        Ok(())
    }

    #[test]
    fn test_calculate_initial_sqrt_price_large_exponent_adjustment() {
        // Test with exponent adjustment causing overflow
        let price_data = create_price(1_000_000_000, 38);
        let decimals_a = 6;
        let decimals_b = 6;

        let result = calculate_initial_sqrt_price(&price_data, decimals_a, decimals_b);

        // Should return MultiplicationOverflowError
        match result {
            Err(e) => {
                if let anchor_lang::error::Error::AnchorError(anchor_error) = e {
                    assert_eq!(
                        anchor_error.error_code_number,
                        ErrorCode::MultiplicationOverflowError as u32 + 6000
                    );
                    assert_eq!(anchor_error.error_name, "MultiplicationOverflowError");
                } else {
                    panic!("Expected AnchorError, got {:?}", e);
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn test_calculate_initial_sqrt_price_large_price() -> Result<()> {
        // Test with maximum possible price
        let price_data = create_price(i64::MAX, -8);
        let decimals_a = 6;
        let decimals_b = 6;

        let result = calculate_initial_sqrt_price(&price_data, decimals_a, decimals_b)?;

        // Manually calculate expected result
        let numerator = price_data.price as u128;
        let denominator = 10u128.pow(8);
        let sqrt_numerator = integer_sqrt(numerator);
        let sqrt_denominator = integer_sqrt(denominator);
        let shifted_numerator = sqrt_numerator << 64;
        let expected_sqrt_price = shifted_numerator / sqrt_denominator;

        assert_eq!(result, expected_sqrt_price);

        Ok(())
    }
}
