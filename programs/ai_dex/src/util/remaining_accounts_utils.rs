use anchor_lang::prelude::*;
use crate::errors::ErrorCode;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum AccountsType {
    TransferHookA,
    TransferHookB,
    TransferHookReward,
    TransferHookInput,
    TransferHookIntermediate,
    TransferHookOutput,
    TransferHookReferralFee,
    //TickArray,
    //TickArrayOne,
    //TickArrayTwo,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RemainingAccountsSlice {
    pub accounts_type: AccountsType,
    pub length: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RemainingAccountsInfo {
    pub slices: Vec<RemainingAccountsSlice>,
}

#[derive(Default)]
pub struct ParsedRemainingAccounts<'info> {
    pub transfer_hook_a: Option<Vec<AccountInfo<'info>>>,
    pub transfer_hook_b: Option<Vec<AccountInfo<'info>>>,
    pub transfer_hook_reward: Option<Vec<AccountInfo<'info>>>,
    pub transfer_hook_input: Option<Vec<AccountInfo<'info>>>,
    pub transfer_hook_intermediate: Option<Vec<AccountInfo<'info>>>,
    pub transfer_hook_output: Option<Vec<AccountInfo<'info>>>,
    pub transfer_hook_referral_fee: Option<Vec<AccountInfo<'info>>>,
}

/// Parses the remaining accounts based on the provided information and valid account types.
///
/// # Arguments
///
/// * `remaining_accounts` - A slice of `AccountInfo` representing the remaining accounts.
/// * `remaining_accounts_info` - An optional reference to `RemainingAccountsInfo` containing slices of account types and lengths.
/// * `valid_accounts_type_list` - A slice of `AccountsType` representing the valid account types.
///
/// # Returns
///
/// Returns a `Result` containing `ParsedRemainingAccounts` if successful, or an error if any validation fails.
///
/// # Errors
///
/// * `ErrorCode::InvalidRemainingAccountsSliceError` - If an account type in the slice is not in the valid accounts type list.
/// * `ErrorCode::InsufficientRemainingAccountsError` - If there are not enough remaining accounts to fulfill the slice length.
/// * `ErrorCode::DuplicateAccountTypesError` - If an account type is duplicated in the parsed remaining accounts.
pub fn parse_remaining_accounts<'info>(
  remaining_accounts: &[AccountInfo<'info>],
  remaining_accounts_info: &Option<RemainingAccountsInfo>,
  valid_accounts_type_list: &[AccountsType],
) -> Result<ParsedRemainingAccounts<'info>> {
  // Create an iterator over the remaining accounts
  let mut remaining_accounts_iter = remaining_accounts.iter();
  // Create a default instance of ParsedRemainingAccounts
  let mut parsed_remaining_accounts = ParsedRemainingAccounts::default();

  // Check if remaining_accounts_info is Some
  if let Some(remaining_accounts_info) = remaining_accounts_info {
    // Iterate over each slice in remaining_accounts_info.slices
    for slice in &remaining_accounts_info.slices {
      // Check if the slice's accounts_type is in the valid_accounts_type_list
      if !valid_accounts_type_list.contains(&slice.accounts_type) {
        return Err(ErrorCode::InvalidRemainingAccountsSliceError.into());
      }
      // Check if the slice's length is 0, if so, skip to the next slice
      if slice.length == 0 {
        continue;
      }

      // Create a vector to store the accounts for this slice
      let mut accounts: Vec<AccountInfo<'info>> = Vec::with_capacity(slice.length as usize);
      // Iterate slice.length times to collect the required number of accounts
      for _ in 0..slice.length {
        // Check if there are remaining accounts, if not, return an error
        if let Some(account) = remaining_accounts_iter.next() {
          accounts.push(account.clone());
        } else {
          return Err(ErrorCode::InsufficientRemainingAccountsError.into());
        }
      }

      // Match the slice's accounts_type 
      // and assign the accounts to the corresponding field in parsed_remaining_accounts
      match slice.accounts_type {
        AccountsType::TransferHookA => {
          if parsed_remaining_accounts.transfer_hook_a.is_some() {
            return Err(ErrorCode::DuplicateAccountTypesError.into());
          }
          parsed_remaining_accounts.transfer_hook_a = Some(accounts);
        }
        AccountsType::TransferHookB => {
          if parsed_remaining_accounts.transfer_hook_b.is_some() {
            return Err(ErrorCode::DuplicateAccountTypesError.into());
          }
          parsed_remaining_accounts.transfer_hook_b = Some(accounts);
        }
        AccountsType::TransferHookReward => {
          if parsed_remaining_accounts.transfer_hook_reward.is_some() {
            return Err(ErrorCode::DuplicateAccountTypesError.into());
          }
          parsed_remaining_accounts.transfer_hook_reward = Some(accounts);
        }
        AccountsType::TransferHookInput => {
          if parsed_remaining_accounts.transfer_hook_input.is_some() {
            return Err(ErrorCode::DuplicateAccountTypesError.into());
          }
          parsed_remaining_accounts.transfer_hook_input = Some(accounts);
        }
        AccountsType::TransferHookIntermediate => {
          if parsed_remaining_accounts.transfer_hook_intermediate.is_some() {
            return Err(ErrorCode::DuplicateAccountTypesError.into());
          }
          parsed_remaining_accounts.transfer_hook_intermediate = Some(accounts);
        }
        AccountsType::TransferHookOutput => {
          if parsed_remaining_accounts.transfer_hook_output.is_some() {
            return Err(ErrorCode::DuplicateAccountTypesError.into());
          }
          parsed_remaining_accounts.transfer_hook_output = Some(accounts);
        },
        AccountsType::TransferHookReferralFee => {
          if parsed_remaining_accounts.transfer_hook_referral_fee.is_some() {
            return Err(ErrorCode::DuplicateAccountTypesError.into());
          }
          parsed_remaining_accounts.transfer_hook_referral_fee = Some(accounts);
        }
      }
    }
  }

  // Return the parsed_remaining_accounts
  Ok(parsed_remaining_accounts)
}
