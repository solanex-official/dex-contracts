use anchor_lang::{
    prelude::{AccountInfo, Pubkey, Signer, *},
    ToAccountInfo,
};
use anchor_spl::token::TokenAccount;
use solana_program::program_option::COption;
use std::convert::TryFrom;

use crate::errors::ErrorCode;

/// Verifies the authority of a position trade batch token account.
///
/// This function checks if the provided position trade batch token account has the correct authority.
/// It uses the same logic as `verify_position_authority`.
///
/// # Arguments
///
/// * `position_trade_batch_token_account` - The position trade batch token account to verify.
/// * `position_trade_batch_authority` - The authority of the position trade batch token account.
///
/// # Errors
///
/// This function returns an error if:
/// * The authority is missing or invalid.
/// * The position token amount is invalid.
pub fn verify_position_trade_batch_authority<'info>(
    position_trade_batch_token_account: &TokenAccount,
    position_trade_batch_authority: &Signer<'info>,
) -> Result<()> {
    // use same logic
    verify_position_authority(position_trade_batch_token_account, position_trade_batch_authority)
}

/// Verifies the authority of a position token account.
///
/// This function checks if the provided position token account has the correct authority.
/// If the position token account has a delegate, it checks if the authority matches the delegate.
/// Otherwise, it checks if the authority matches the owner.
///
/// # Arguments
///
/// * `position_token_account` - The position token account to verify.
/// * `position_authority` - The authority of the position token account.
///
/// # Errors
///
/// This function returns an error if:
/// * The authority is missing or invalid.
/// * The position token amount is invalid.
pub fn verify_position_authority<'info>(
    position_token_account: &TokenAccount,
    position_authority: &Signer<'info>,
) -> Result<()> {
    if let COption::Some(ref delegate) = position_token_account.delegate {
        if position_authority.key == delegate {
            validate_owner(delegate, &position_authority.to_account_info())?;
            if position_token_account.delegated_amount != 1 {
                return Err(ErrorCode::InvalidPositionTokenAmountError.into());
            }
        } else {
            validate_owner(
                &position_token_account.owner,
                &position_authority.to_account_info(),
            )?;
        }
    } else {
        validate_owner(
            &position_token_account.owner,
            &position_authority.to_account_info(),
        )?;
    }
    Ok(())
}

/// Validates the owner of an account.
///
/// This function checks if the provided owner matches the expected owner and if the owner is a signer.
///
/// # Arguments
///
/// * `expected_owner` - The expected owner of the account.
/// * `owner_account_info` - The account info of the owner.
///
/// # Errors
///
/// This function returns an error if the owner is missing or invalid.
fn validate_owner(expected_owner: &Pubkey, owner_account_info: &AccountInfo) -> Result<()> {
    if expected_owner != owner_account_info.key || !owner_account_info.is_signer {
        return Err(ErrorCode::InvalidDelegateError.into());
    }

    Ok(())
}

/// Converts a timestamp from `i64` to `u64`.
///
/// This function converts a timestamp from `i64` to `u64`.
///
/// # Arguments
///
/// * `t` - The timestamp to convert.
///
/// # Errors
///
/// This function returns an error if the timestamp conversion is invalid.
pub fn to_timestamp_u64(t: i64) -> Result<u64> {
    u64::try_from(t).or(Err(ErrorCode::TimestampConversionError.into()))
}
