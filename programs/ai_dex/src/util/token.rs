use crate::state::{AiDexPool, PositionTradeBatch, SwapReferral};
use crate::errors::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint as SplMint, Token, TokenAccount as SplTokenAccount};
use anchor_spl::metadata::{self, CreateMetadataAccountsV3, mpl_token_metadata::types::DataV2};
use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::{TransferFee, MAX_FEE_BASIS_POINTS};
use anchor_spl::token_interface::spl_token_2022::extension::BaseStateWithExtensions;
use anchor_spl::token_2022::spl_token_2022::{self, extension::{self, StateWithExtensions}, state::AccountState};
use anchor_spl::token_interface::{Mint as InterfaceMint, TokenAccount as InterfaceTokenAccount, TokenInterface};
use anchor_spl::memo::{self, Memo, BuildMemo};
use spl_transfer_hook_interface;
use solana_program::program::invoke_signed;
use spl_token::instruction::{burn_checked, close_account, mint_to, set_authority, AuthorityType};

use crate::constants::nft::{
    ADB_METADATA_SYMBOL, ADB_METADATA_URI, AD_METADATA_NAME,
    AD_METADATA_SYMBOL, AD_METADATA_URI,
};

/// Burns a single token from the user's position token account and closes the account.
///
/// # Arguments
///
/// * `token_authority` - The signer authority for the token.
/// * `receiver` - The account to receive the remaining funds.
/// * `position_mint` - The mint of the position token.
/// * `position_token_account` - The user's position token account.
/// * `token_program` - The token program.
///
/// # Errors
///
/// Returns an error if the burn or close account operations fail.
pub fn burn_and_close_user_position_token<'info>(
    token_authority: &Signer<'info>,
    receiver: &UncheckedAccount<'info>,
    position_mint: &Account<'info, SplMint>,
    position_token_account: &Account<'info, SplTokenAccount>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    // Burn a single token in user account
    invoke_signed(
        &burn_checked(
            token_program.key,
            position_token_account.to_account_info().key,
            position_mint.to_account_info().key,
            token_authority.key,
            &[],
            1,
            position_mint.decimals,
        )?,
        &[
            token_program.to_account_info(),
            position_token_account.to_account_info(),
            position_mint.to_account_info(),
            token_authority.to_account_info(),
        ],
        &[],
    )?;

    // Close user account
    invoke_signed(
        &close_account(
            token_program.key,
            position_token_account.to_account_info().key,
            receiver.key,
            token_authority.key,
            &[],
        )?,
        &[
            token_program.to_account_info(),
            position_token_account.to_account_info(),
            receiver.to_account_info(),
            token_authority.to_account_info(),
        ],
        &[],
    )?;
    Ok(())
}


/// Mints a position token and removes the mint authority.
///
/// # Arguments
///
/// * `ai_dex` - The AiDex account.
/// * `position_mint` - The mint of the position token.
/// * `position_token_account` - The position token account.
/// * `token_program` - The token program.
///
/// # Errors
///
/// Returns an error if the mint or authority removal fails.
pub fn mint_position_token_and_remove_authority<'info>(
    ai_dex: &AccountLoader<'info, AiDexPool>,
    position_mint: &Account<'info, SplMint>,
    position_token_account: &Account<'info, SplTokenAccount>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    mint_position_token(
        ai_dex,
        position_mint,
        position_token_account,
        token_program,
    )?;
    remove_position_token_mint_authority(ai_dex, position_mint, token_program)
}

/// Mints a position token with metadata and removes the mint authority.
///
/// # Arguments
///
/// * `ai_dex` - The AiDex account.
/// * `position_mint` - The mint of the position token.
/// * `position_token_account` - The position token account.
/// * `position_metadata_account` - The position metadata account.
/// * `metadata_update_auth` - The metadata update authority.
/// * `funder` - The funder of the metadata account.
/// * `metadata_program` - The metadata program.
/// * `token_program` - The token program.
/// * `system_program` - The system program.
/// * `rent` - The rent sysvar.
///
/// # Errors
///
/// Returns an error if the mint, metadata creation, or authority removal fails.
pub fn mint_position_token_with_metadata_and_remove_authority<'info>(
    ai_dex: &AccountLoader<'info, AiDexPool>,
    position_mint: &Account<'info, SplMint>,
    position_token_account: &Account<'info, SplTokenAccount>,
    position_metadata_account: &UncheckedAccount<'info>,
    metadata_update_auth: &UncheckedAccount<'info>,
    funder: &Signer<'info>,
    metadata_program: &Program<'info, metadata::Metadata>,
    token_program: &Program<'info, Token>,
    system_program: &Program<'info, System>,
    rent: &Sysvar<'info, Rent>,
) -> Result<()> {
    mint_position_token(
        ai_dex,
        position_mint,
        position_token_account,
        token_program,
    )?;

    let metadata_mint_auth_account = ai_dex.load()?;
    metadata::create_metadata_accounts_v3(
        CpiContext::new_with_signer(
            metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: position_metadata_account.to_account_info(),
                mint: position_mint.to_account_info(),
                mint_authority: ai_dex.to_account_info(),
                update_authority: metadata_update_auth.to_account_info(),
                payer: funder.to_account_info(),
                rent: rent.to_account_info(),
                system_program: system_program.to_account_info(),
            },
            &[&metadata_mint_auth_account.seeds()],
        ),
        DataV2 {
            name: AD_METADATA_NAME.to_string(),
            symbol: AD_METADATA_SYMBOL.to_string(),
            uri: AD_METADATA_URI.to_string(),
            creators: None,
            seller_fee_basis_points: 0,
            collection: None,
            uses: None,
        },
        true,
        false,
        None,
    )?;

    remove_position_token_mint_authority(ai_dex, position_mint, token_program)
}

/// Mints a single position token to the specified token account.
///
/// # Arguments
///
/// * `ai_dex` - The AiDex account which has the authority to mint the token.
/// * `position_mint` - The mint of the position token.
/// * `position_token_account` - The account to receive the minted token.
/// * `token_program` - The token program.
///
/// # Errors
///
/// Returns an error if the mint operation fails.
fn mint_position_token<'info>(
    ai_dex: &AccountLoader<'info, AiDexPool>,
    position_mint: &Account<'info, SplMint>,
    position_token_account: &Account<'info, SplTokenAccount>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    let ai_dex_data = ai_dex.load()?;
    invoke_signed(
        &mint_to(
            token_program.key,
            position_mint.to_account_info().key,
            position_token_account.to_account_info().key,
            ai_dex.to_account_info().key,
            &[ai_dex.to_account_info().key],
            1,
        )?,
        &[
            position_mint.to_account_info(),
            position_token_account.to_account_info(),
            ai_dex.to_account_info(),
            token_program.to_account_info(),
        ],
        &[&ai_dex_data.seeds()],
    )?;
    Ok(())
}

/// Removes the mint authority from the position token.
///
/// # Arguments
///
/// * `ai_dex` - The AiDex account.
/// * `position_mint` - The mint of the position token.
/// * `token_program` - The token program.
///
/// # Errors
///
/// Returns an error if the authority removal fails.
fn remove_position_token_mint_authority<'info>(
    ai_dex: &AccountLoader<'info, AiDexPool>,
    position_mint: &Account<'info, SplMint>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    let ai_dex_data = ai_dex.load()?; 
    invoke_signed(
        &set_authority(
            token_program.key,
            position_mint.to_account_info().key,
            Option::None,
            AuthorityType::MintTokens,
            ai_dex.to_account_info().key,
            &[ai_dex.to_account_info().key],
        )?,
        &[
            position_mint.to_account_info(),
            ai_dex.to_account_info(),
            token_program.to_account_info(),
        ],
        &[&ai_dex_data.seeds()],
    )?;
    Ok(())
}

/// Mints a position trade batch token and then removes the mint authority.
///
/// # Arguments
///
/// * `position_trade_batch` - The account representing the position trade batch.
/// * `position_trade_batch_mint` - The mint of the position trade batch token.
/// * `position_trade_batch_token_account` - The token account for the position trade batch.
/// * `token_program` - The token program.
/// * `position_trade_batch_seeds` - The seeds for the position trade batch.
///
/// # Errors
///
/// Returns an error if the mint operation or the removal of the mint authority fails.
pub fn mint_position_trade_batch_token_and_remove_authority<'info>(
    position_trade_batch: &Account<'info, PositionTradeBatch>,
    position_trade_batch_mint: &Account<'info, SplMint>,
    position_trade_batch_token_account: &Account<'info, SplTokenAccount>,
    token_program: &Program<'info, Token>,
    position_trade_batch_seeds: &[&[u8]],
) -> Result<()> {
    mint_position_trade_batch_token(
        position_trade_batch,
        position_trade_batch_mint,
        position_trade_batch_token_account,
        token_program,
        position_trade_batch_seeds,
    )?;
    remove_position_trade_batch_token_mint_authority(
        position_trade_batch,
        position_trade_batch_mint,
        token_program,
        position_trade_batch_seeds,
    )
}

/// Mints a position trade batch token with metadata and removes the mint authority.
///
/// # Arguments
///
/// * `funder` - The account funding the transaction.
/// * `position_trade_batch` - The position trade batch account.
/// * `position_trade_batch_mint` - The mint of the position trade batch token.
/// * `position_trade_batch_token_account` - The position trade batch token account.
/// * `position_trade_batch_metadata` - The metadata account for the position trade batch token.
/// * `metadata_update_auth` - The account authorized to update the metadata.
/// * `metadata_program` - The metadata program.
/// * `token_program` - The token program.
/// * `system_program` - The system program.
/// * `rent` - The rent sysvar.
/// * `position_trade_batch_seeds` - The seeds for the position trade batch.
///
/// # Errors
///
/// Returns an error if the mint, metadata creation, or authority removal fails.
pub fn mint_position_trade_batch_token_with_metadata_and_remove_authority<'info>(
    funder: &Signer<'info>,
    position_trade_batch: &Account<'info, PositionTradeBatch>,
    position_trade_batch_mint: &Account<'info, SplMint>,
    position_trade_batch_token_account: &Account<'info, SplTokenAccount>,
    position_trade_batch_metadata: &UncheckedAccount<'info>,
    metadata_update_auth: &UncheckedAccount<'info>,
    metadata_program: &Program<'info, metadata::Metadata>,
    token_program: &Program<'info, Token>,
    system_program: &Program<'info, System>,
    rent: &Sysvar<'info, Rent>,
    position_trade_batch_seeds: &[&[u8]],
) -> Result<()> {
    mint_position_trade_batch_token(
        position_trade_batch,
        position_trade_batch_mint,
        position_trade_batch_token_account,
        token_program,
        position_trade_batch_seeds,
    )?;

    let mint_address = position_trade_batch_mint.key().to_string();
    let nft_name = format!(
        "{} {}...{}",
        ADB_METADATA_SYMBOL,
        &mint_address[0..4],
        &mint_address[mint_address.len() - 4..]
    );

    metadata::create_metadata_accounts_v3(
        CpiContext::new_with_signer(
            metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: position_trade_batch_metadata.to_account_info(),
                mint: position_trade_batch_mint.to_account_info(),
                mint_authority: position_trade_batch.to_account_info(),
                update_authority: metadata_update_auth.to_account_info(),
                payer: funder.to_account_info(),
                rent: rent.to_account_info(),
                system_program: system_program.to_account_info(),
            },
            &[position_trade_batch_seeds],
        ),
        DataV2 {
            name: nft_name,
            symbol: ADB_METADATA_SYMBOL.to_string(),
            uri: ADB_METADATA_URI.to_string(),
            creators: None,
            seller_fee_basis_points: 0,
            collection: None,
            uses: None
        },
        true,
        false,
        None
    )?;

    remove_position_trade_batch_token_mint_authority(
        position_trade_batch,
        position_trade_batch_mint,
        token_program,
        position_trade_batch_seeds,
    )
}

/// Mints a position trade batch token.
///
/// # Arguments
///
/// * `position_trade_batch` - The account representing the position trade batch.
/// * `position_trade_batch_mint` - The mint of the position trade batch token.
/// * `position_trade_batch_token_account` - The token account for the position trade batch.
/// * `token_program` - The token program.
/// * `position_trade_batch_seeds` - The seeds for the position trade batch.
///
/// # Errors
///
/// Returns an error if the mint operation fails.
fn mint_position_trade_batch_token<'info>(
    position_trade_batch: &Account<'info, PositionTradeBatch>,
    position_trade_batch_mint: &Account<'info, SplMint>,
    position_trade_batch_token_account: &Account<'info, SplTokenAccount>,
    token_program: &Program<'info, Token>,
    position_trade_batch_seeds: &[&[u8]],
) -> Result<()> {
    invoke_signed(
        &mint_to(
            token_program.key,
            position_trade_batch_mint.to_account_info().key,
            position_trade_batch_token_account.to_account_info().key,
            position_trade_batch.to_account_info().key,
            &[],
            1,
        )?,
        &[
            position_trade_batch_mint.to_account_info(),
            position_trade_batch_token_account.to_account_info(),
            position_trade_batch.to_account_info(),
            token_program.to_account_info(),
        ],
        &[position_trade_batch_seeds],
    )?;

    Ok(())
}

/// Removes the mint authority from the position trade batch token.
///
/// # Arguments
///
/// * `position_trade_batch` - The PositionTradeBatch account.
/// * `position_trade_batch_mint` - The mint of the position trade batch token.
/// * `token_program` - The token program.
/// * `position_trade_batch_seeds` - The seeds for the position trade batch.
///
/// # Errors
///
/// Returns an error if the authority removal fails.
fn remove_position_trade_batch_token_mint_authority<'info>(
    position_trade_batch: &Account<'info, PositionTradeBatch>,
    position_trade_batch_mint: &Account<'info, SplMint>,
    token_program: &Program<'info, Token>,
    position_trade_batch_seeds: &[&[u8]],
) -> Result<()> {
    // Invoke the set_authority instruction to remove the mint authority
    invoke_signed(
        &set_authority(
            token_program.key,
            position_trade_batch_mint.to_account_info().key,
            Option::None,
            AuthorityType::MintTokens,
            position_trade_batch.to_account_info().key,
            &[],
        )?,
        &[
            position_trade_batch_mint.to_account_info(),
            position_trade_batch.to_account_info(),
            token_program.to_account_info(),
        ],
        &[position_trade_batch_seeds],
    )?;

    Ok(())
}

/// Burns a single token from the position trade batch token account and closes the account.
///
/// # Arguments
///
/// * `position_trade_batch_authority` - The signer authority for the position trade batch.
/// * `receiver` - The account to receive the remaining funds.
/// * `position_trade_batch_mint` - The mint of the position trade batch token.
/// * `position_trade_batch_token_account` - The position trade batch token account.
/// * `token_program` - The token program.
///
/// # Errors
///
/// Returns an error if the burn or close account operations fail.
pub fn burn_and_close_position_trade_batch_token<'info>(
    position_trade_batch_authority: &Signer<'info>,
    receiver: &UncheckedAccount<'info>,
    position_trade_batch_mint: &Account<'info, SplMint>,
    position_trade_batch_token_account: &Account<'info, SplTokenAccount>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    // use same logic
    burn_and_close_user_position_token(
        position_trade_batch_authority,
        receiver,
        position_trade_batch_mint,
        position_trade_batch_token_account,
        token_program,
    )
}

/// Transfers tokens from the owner's account to the vault.
///
/// This function performs the following steps:
/// 1. Checks for and logs any applicable transfer fees using the memo program.
/// 2. Creates a transfer instruction using the `spl_token_2022::instruction::transfer_checked` function.
/// 3. Prepares the necessary account infos for the transfer instruction.
/// 4. Handles any transfer hooks by adding extra accounts if required.
/// 5. Invokes the transfer instruction.
///
/// # Arguments
///
/// * `authority` - A reference to the signer authority.
/// * `token_mint` - A reference to the token mint account.
/// * `token_owner_account` - A reference to the token owner's token account.
/// * `token_vault` - A reference to the token vault account.
/// * `token_program` - A reference to the token program interface.
/// * `memo_program` - A reference to the memo program.
/// * `transfer_hook_accounts` - An optional vector of additional accounts for transfer hooks.
/// * `amount` - The amount of tokens to transfer.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the transfer is successful, otherwise returns an error.
///
/// # Errors
///
/// Returns an error if there is an issue with logging the transfer fee, creating the transfer instruction,
/// preparing the account infos, handling the transfer hooks, or invoking the transfer instruction.
pub fn transfer_from_owner_to_vault<'info>(
    authority: &Signer<'info>,
    token_mint: &InterfaceAccount<'info, InterfaceMint>,
    token_owner_account: &InterfaceAccount<'info, InterfaceTokenAccount>,
    token_vault: &InterfaceAccount<'info, InterfaceTokenAccount>,
    token_program: &Interface<'info, TokenInterface>,
    memo_program: &Program<'info, Memo>,
    transfer_hook_accounts: &Option<Vec<AccountInfo<'info>>>,
    amount: u64,
) -> Result<()> {
    // Handle TransferFee extension
    if let Some(epoch_transfer_fee) = get_epoch_transfer_fee(token_mint)? {
        // log applied transfer fee
        // - Not must, but important for ease of investigation and replay when problems occur
        // - Use Memo because logs risk being truncated
        let transfer_fee_memo = format!(
            "TFe: {}, {}",
            u16::from(epoch_transfer_fee.transfer_fee_basis_points),
            u64::from(epoch_transfer_fee.maximum_fee),
        );
        memo::build_memo(
            CpiContext::new(
                memo_program.to_account_info(),
                BuildMemo {}
            ),
            transfer_fee_memo.as_bytes()
        )?;
    }

    // Create transfer instruction
    let mut instruction = spl_token_2022::instruction::transfer_checked(
        token_program.key,
        &token_owner_account.key(), // from
        &token_mint.key(), // mint
        &token_vault.key(), // to
        authority.key, // authority
        &[],
        amount,
        token_mint.decimals,
    )?;

    // Prepare account infos
    let mut account_infos = vec![
        token_program.to_account_info(),
        token_owner_account.to_account_info(),
        token_mint.to_account_info(),
        token_vault.to_account_info(),
        authority.to_account_info(),
    ];

    // Handle TransferHook extension
    if let Some(hook_program_id) = get_transfer_hook_program_id(token_mint)? {
        if let Some(hook_accounts) = transfer_hook_accounts {
            spl_transfer_hook_interface::onchain::add_extra_accounts_for_execute_cpi(
                &mut instruction,
                &mut account_infos,
                &hook_program_id,
                token_owner_account.to_account_info(),
                token_mint.to_account_info(),
                token_vault.to_account_info(),
                authority.to_account_info(),
                amount,
                hook_accounts,
            )?;
        } else {
            return Err(ErrorCode::MissingExtraAccountsForTransferHookError.into());
        }
    }

    // Invoke the instruction
    solana_program::program::invoke_signed(
        &instruction,
        &account_infos,
        &[],
    )?;

    Ok(())
}

/// Builds and logs a memo using the provided memo program and content.
///
/// This function constructs a memo using the `memo::build_memo` function and logs it
/// using the provided memo program.
///
/// # Arguments
///
/// * `memo_program` - A reference to the memo program.
/// * `memo_content` - The content of the memo to be logged.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the memo is successfully built and logged, otherwise returns an error.
///
/// # Errors
///
/// Returns an error if there is an issue with building or logging the memo.
fn build_and_log_memo<'info>(
    memo_program: &Program<'info, Memo>,
    memo_content: &[u8],
) -> Result<()> {
    memo::build_memo(
        CpiContext::new(
            memo_program.to_account_info(),
            BuildMemo {},
        ),
        memo_content,
    )
}

/// Transfers tokens from the vault to the owner's account.
///
/// This function performs the following steps:
/// 1. Checks for and logs any applicable transfer fees using the memo program.
/// 2. Logs a memo if required by the token owner's account.
/// 3. Creates a transfer instruction using the `spl_token_2022::instruction::transfer_checked` function.
/// 4. Prepares the necessary account infos for the transfer instruction.
/// 5. Handles any transfer hooks by adding extra accounts if required.
/// 6. Invokes the transfer instruction.
///
/// # Arguments
///
/// * `ai_dex` - A reference to the AiDex account.
/// * `token_mint` - A reference to the token mint account.
/// * `token_vault` - A reference to the token vault account.
/// * `token_owner_account` - A reference to the token owner's token account.
/// * `token_program` - A reference to the token program interface.
/// * `memo_program` - A reference to the memo program.
/// * `transfer_hook_accounts` - An optional vector of additional accounts for transfer hooks.
/// * `amount` - The amount of tokens to transfer.
/// * `memo` - The memo to be logged if required.
///
/// # Returns
///
/// * `Result<()>` - Returns `Ok(())` if the transfer is successful, otherwise returns an error.
///
/// # Errors
///
/// Returns an error if there is an issue with logging the transfer fee, logging the memo,
/// creating the transfer instruction, preparing the account infos, handling the transfer hooks,
/// or invoking the transfer instruction.
pub fn transfer_from_vault_to_owner<'info>(
    ai_dex: &AccountLoader<'info, AiDexPool>,
    token_mint: &InterfaceAccount<'info, InterfaceMint>,
    token_vault: &InterfaceAccount<'info, InterfaceTokenAccount>,
    token_owner_account: &InterfaceAccount<'info, InterfaceTokenAccount>,
    token_program: &Interface<'info, TokenInterface>,
    memo_program: &Program<'info, Memo>,
    transfer_hook_accounts: &Option<Vec<AccountInfo<'info>>>,
    amount: u64,
    memo: &[u8],
) -> Result<()> {
    // Handle TransferFee extension
    if let Some(epoch_transfer_fee) = get_epoch_transfer_fee(token_mint)? {
        let transfer_fee_memo = format!(
            "TFe: {}, {}",
            u16::from(epoch_transfer_fee.transfer_fee_basis_points),
            u64::from(epoch_transfer_fee.maximum_fee),
        );
        build_and_log_memo(memo_program, transfer_fee_memo.as_bytes())?;
    }

    // Handle MemoTransfer extension
    if is_transfer_memo_required(&token_owner_account)? {
        build_and_log_memo(memo_program, memo)?;
    }

    // Create transfer instruction
    let mut instruction = spl_token_2022::instruction::transfer_checked(
        token_program.key,
        &token_vault.key(), // from
        &token_mint.key(), // mint
        &token_owner_account.key(), // to
        &ai_dex.key(), // authority
        &[],
        amount,
        token_mint.decimals,
    )?;

    // Prepare account infos
    let mut account_infos = vec![
        token_program.to_account_info(),
        token_vault.to_account_info(),
        token_mint.to_account_info(),
        token_owner_account.to_account_info(),
        ai_dex.to_account_info(),
    ];

    // Handle TransferHook extension
    if let Some(hook_program_id) = get_transfer_hook_program_id(token_mint)? {
        if let Some(hook_accounts) = transfer_hook_accounts {
            spl_transfer_hook_interface::onchain::add_extra_accounts_for_execute_cpi(
                &mut instruction,
                &mut account_infos,
                &hook_program_id,
                token_owner_account.to_account_info(),
                token_mint.to_account_info(),
                token_vault.to_account_info(),
                ai_dex.to_account_info(),
                amount,
                hook_accounts,
            )?;
        } else {
            return Err(ErrorCode::MissingExtraAccountsForTransferHookError.into());
        }
    }

    let ai_dex_data = ai_dex.load()?; 

    // Invoke the instruction
    solana_program::program::invoke_signed(
        &instruction,
        &account_infos,
        &[&ai_dex_data.seeds()],
    )?;

    drop(ai_dex_data);

    Ok(())
}

pub fn transfer_from_referral_to_owner<'info>(
    referral_swap: &Account<'info, SwapReferral>,
    token_mint: &InterfaceAccount<'info, InterfaceMint>,
    token_vault: &InterfaceAccount<'info, InterfaceTokenAccount>,
    token_owner_account: &InterfaceAccount<'info, InterfaceTokenAccount>,
    token_program: &Interface<'info, TokenInterface>,
    memo_program: &Program<'info, Memo>,
    transfer_hook_accounts: &Option<Vec<AccountInfo<'info>>>,
    amount: u64,
    memo: &[u8],
) -> Result<()> {
    // Handle TransferFee extension
    if let Some(epoch_transfer_fee) = get_epoch_transfer_fee(token_mint)? {
        let transfer_fee_memo = format!(
            "TFe: {}, {}",
            u16::from(epoch_transfer_fee.transfer_fee_basis_points),
            u64::from(epoch_transfer_fee.maximum_fee),
        );
        build_and_log_memo(memo_program, transfer_fee_memo.as_bytes())?;
    }

    // Handle MemoTransfer extension
    if is_transfer_memo_required(&token_owner_account)? {
        build_and_log_memo(memo_program, memo)?;
    }

    // Create transfer instruction
    let mut instruction = spl_token_2022::instruction::transfer_checked(
        token_program.key,
        &token_vault.key(), // from
        &token_mint.key(), // mint
        &token_owner_account.key(), // to
        &referral_swap.key(), // authority
        &[],
        amount,
        token_mint.decimals,
    )?;

    // Prepare account infos
    let mut account_infos = vec![
        token_program.to_account_info(),
        token_vault.to_account_info(),
        token_mint.to_account_info(),
        token_owner_account.to_account_info(),
        referral_swap.to_account_info(),
    ];

    // Handle TransferHook extension
    if let Some(hook_program_id) = get_transfer_hook_program_id(token_mint)? {
        if let Some(hook_accounts) = transfer_hook_accounts {
            spl_transfer_hook_interface::onchain::add_extra_accounts_for_execute_cpi(
                &mut instruction,
                &mut account_infos,
                &hook_program_id,
                token_owner_account.to_account_info(),
                token_mint.to_account_info(),
                token_vault.to_account_info(),
                referral_swap.to_account_info(),
                amount,
                hook_accounts,
            )?;
        } else {
            return Err(ErrorCode::MissingExtraAccountsForTransferHookError.into());
        }
    }

    // Invoke the instruction
    solana_program::program::invoke_signed(
        &instruction,
        &account_infos,
        &[&referral_swap.seeds()],
    )?;

    Ok(())
}

/// Retrieves the transfer hook program ID for a given token mint.
///
/// This function checks if the token mint is owned by the Token Program and, if not,
/// retrieves the transfer hook program ID from the token mint's extensions.
///
/// # Arguments
///
/// * `token_mint` - A reference to the token mint account.
///
/// # Returns
///
/// * `Result<Option<Pubkey>>` - Returns `Ok(Some(Pubkey))` if a transfer hook program ID is found,
///   `Ok(None)` if the token mint is owned by the Token Program, otherwise returns an error.
///
/// # Errors
///
/// Returns an error if there is an issue with borrowing data or unpacking the mint data.
fn get_transfer_hook_program_id<'info>(
    token_mint: &InterfaceAccount<'info, InterfaceMint>,
) -> Result<Option<Pubkey>> {
    let token_mint_info = token_mint.to_account_info();
    if *token_mint_info.owner == Token::id() {
        return Ok(None);
    }

    let token_mint_data = token_mint_info.try_borrow_data()?;
    let token_mint_unpacked = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint_data)?;
    Ok(extension::transfer_hook::get_program_id(&token_mint_unpacked))
}

/// Checks if a transfer memo is required for a given token account.
///
/// This function checks if the token account is owned by the Token Program and, if not,
/// retrieves the memo transfer extension to determine if incoming transfer memos are required.
///
/// # Arguments
///
/// * `token_account` - A reference to the token account.
///
/// # Returns
///
/// * `Result<bool>` - Returns `Ok(true)` if incoming transfer memos are required,
///   `Ok(false)` if the token account is owned by the Token Program or if the memo transfer extension is not found.
///
/// # Errors
///
/// Returns an error if there is an issue with borrowing data or unpacking the account data.
fn is_transfer_memo_required<'info>(token_account: &InterfaceAccount<'info, InterfaceTokenAccount>) -> Result<bool> {
    let token_account_info = token_account.to_account_info();
    if *token_account_info.owner == Token::id() {
        return Ok(false);
    }

    let token_account_data = token_account_info.try_borrow_data()?;
    let token_account_unpacked = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&token_account_data)?;
    let extension = token_account_unpacked.get_extension::<extension::memo_transfer::MemoTransfer>();

    if let Ok(memo_transfer) = extension {
        return Ok(memo_transfer.require_incoming_transfer_memos.into());
    } else {
        return Ok(false);
    }
}

/// Checks if the given token mint is supported.
///
/// This function performs several checks to determine if a token mint is supported:
/// 1. Checks if the mint is owned by the Token Program.
/// 2. Checks if the mint is the native mint of the Token-2022 Program.
/// 4. Unpacks the mint data and iterates over the extension types to handle each case accordingly.
///
/// # Arguments
///
/// * `token_mint` - A reference to the token mint account.
///
/// # Returns
///
/// * `Result<bool>` - Returns `Ok(true)` if the token mint is supported, otherwise returns `Ok(false)`.
///
/// # Errors
///
/// Returns an error if there is an issue with borrowing data or unpacking the mint data.
pub fn is_supported_token_mint<'info>(
    token_mint: &InterfaceAccount<'info, InterfaceMint>,
) -> Result<bool> {
    let token_mint_info = token_mint.to_account_info();

    // Check if mint is owned by the Token Program
    if *token_mint_info.owner == Token::id() {
        return Ok(true);
    }

    // Check if mint is the native mint of the Token-2022 Program
    if spl_token_2022::native_mint::check_id(&token_mint.key()) {
        return Ok(false);
    }

    let token_mint_data = token_mint_info.try_borrow_data()?;
    let token_mint_unpacked = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint_data)?;
    let extensions = token_mint_unpacked.get_extension_types()?;

    for extension in extensions {
        match extension {
            // supported
            extension::ExtensionType::TransferFeeConfig |
            extension::ExtensionType::TokenMetadata |
            extension::ExtensionType::MetadataPointer => {
                // Supported extensions
            }
            // Supported, but non-confidential transfer only
            //
            // AiDexProgram invokes TransferChecked instruction and it supports non-confidential transfer only.
            //
            // Because the vault accounts are not configured to support confidential transfer,
            // it is impossible to send tokens directly to the vault accounts confidentially.
            // Note: Only the owner (AiDex account) can call ConfidentialTransferInstruction::ConfigureAccount.
            extension::ExtensionType::ConfidentialTransferMint |
            
            extension::ExtensionType::ConfidentialTransferFeeConfig => {
                // Supported, but non-confidential transfer only
                // When both TransferFeeConfig and ConfidentialTransferMint are initialized,
                // ConfidentialTransferFeeConfig is also initialized to store encrypted transfer fee amount.
            }
            extension::ExtensionType::PermanentDelegate |
            extension::ExtensionType::TransferHook |
            extension::ExtensionType::MintCloseAuthority |
            extension::ExtensionType::DefaultAccountState => {

                // reject if default state is not Initialized
                if let extension::ExtensionType::DefaultAccountState = extension {
                    let default_state = token_mint_unpacked.get_extension::<extension::default_account_state::DefaultAccountState>()?;
                    let initialized: u8 = AccountState::Initialized.into();
                    if default_state.state != initialized {
                        return Ok(false);
                    }
                }
            }
            // No possibility to support the following extensions
            extension::ExtensionType::NonTransferable => {
                return Ok(false);
            }
            // mint has unknown or unsupported extensions
            _ => {
                return Ok(false);
            }
        }
    }

    return Ok(true);
}

#[derive(Debug)]
pub struct TransferFeeIncludedAmount {
    pub amount: u64,
    pub transfer_fee: u64,
}

#[derive(Debug)]
pub struct TransferFeeExcludedAmount {
    pub amount: u64,
    pub transfer_fee: u64,
}

pub fn calculate_transfer_fee_excluded_amount<'info>(
    token_mint: &InterfaceAccount<'info, InterfaceMint>,
    transfer_fee_included_amount: u64,
) -> Result<TransferFeeExcludedAmount> {
    if let Some(epoch_transfer_fee) = get_epoch_transfer_fee(token_mint)? {
        let transfer_fee = epoch_transfer_fee.calculate_fee(transfer_fee_included_amount).unwrap();
        let transfer_fee_excluded_amount = transfer_fee_included_amount.checked_sub(transfer_fee).unwrap();
        return Ok(TransferFeeExcludedAmount { amount: transfer_fee_excluded_amount, transfer_fee });            
    }

    Ok(TransferFeeExcludedAmount { amount: transfer_fee_included_amount, transfer_fee: 0 })
} 

pub fn calculate_transfer_fee_included_amount<'info>(
    token_mint: &InterfaceAccount<'info, InterfaceMint>,
    transfer_fee_excluded_amount: u64,
) -> Result<TransferFeeIncludedAmount> {
    if transfer_fee_excluded_amount == 0 {
        return Ok(TransferFeeIncludedAmount { amount: 0, transfer_fee: 0 });
    }

    // now transfer_fee_excluded_amount > 0

    if let Some(epoch_transfer_fee) = get_epoch_transfer_fee(token_mint)? {
        let transfer_fee: u64 = if u16::from(epoch_transfer_fee.transfer_fee_basis_points) == MAX_FEE_BASIS_POINTS {
            // edge-case: if transfer fee rate is 100%, current SPL implementation returns 0 as inverse fee.
            // https://github.com/solana-labs/solana-program-library/blob/fe1ac9a2c4e5d85962b78c3fc6aaf028461e9026/token/program-2022/src/extension/transfer_fee/mod.rs#L95
            
            // But even if transfer fee is 100%, we can use maximum_fee as transfer fee.
            // if transfer_fee_excluded_amount + maximum_fee > u64 max, the following checked_add should fail.
            u64::from(epoch_transfer_fee.maximum_fee)
        } else {
            epoch_transfer_fee.calculate_inverse_fee(transfer_fee_excluded_amount)
                .ok_or(ErrorCode::TransferFeeCalculationError)?
        };

        let transfer_fee_included_amount = transfer_fee_excluded_amount.checked_add(transfer_fee)
            .ok_or(ErrorCode::TransferFeeCalculationError)?;

        // verify transfer fee calculation for safety
        let transfer_fee_verification = epoch_transfer_fee.calculate_fee(transfer_fee_included_amount).unwrap();
        if transfer_fee != transfer_fee_verification {
            // We believe this should never happen
            return Err(ErrorCode::TransferFeeCalculationError.into());
        }

        return Ok(TransferFeeIncludedAmount { amount: transfer_fee_included_amount, transfer_fee });
    }

    Ok(TransferFeeIncludedAmount { amount: transfer_fee_excluded_amount, transfer_fee: 0 })
}

pub fn get_epoch_transfer_fee<'info>(
    token_mint: &InterfaceAccount<'info, InterfaceMint>,
) -> Result<Option<TransferFee>> {
    let token_mint_info = token_mint.to_account_info();
    if *token_mint_info.owner == Token::id() {
        return Ok(None);
    }

    let token_mint_data = token_mint_info.try_borrow_data()?;
    let token_mint_unpacked = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint_data)?;
    if let Ok(transfer_fee_config) = token_mint_unpacked.get_extension::<extension::transfer_fee::TransferFeeConfig>() {
        let epoch = Clock::get()?.epoch;
        return Ok(Some(transfer_fee_config.get_epoch_fee(epoch).clone()));
    }

    Ok(None)
}

#[cfg(test)]
mod fuzz_tests {
    use proptest::prelude::*;
    use super::*;

    struct SyscallStubs {}
    impl solana_program::program_stubs::SyscallStubs for SyscallStubs {
        fn sol_get_clock_sysvar(&self, _var_addr: *mut u8) -> u64 {
            0
        }
    }

    #[derive(Default, AnchorSerialize)]
    struct MintWithTransferFeeConfigLayout {
        // 82 for Mint
        pub coption_mint_authority: u32, // 4
        pub mint_authority: Pubkey, // 32
        pub supply: u64, // 8
        pub decimals: u8, // 1
        pub is_initialized: bool, // 1
        pub coption_freeze_authority: u32, // 4
        pub freeze_authority: Pubkey, // 4 + 32

        // 83 for padding
        pub padding1: [u8; 32],
        pub padding2: [u8; 32],
        pub padding3: [u8; 19],

        pub account_type: u8, // 1

        pub extension_type: u16, // 2
        pub extension_length: u16, // 2
        // 108 for TransferFeeConfig data
        pub transfer_fee_config_authority: Pubkey, // 32
        pub withdraw_withheld_authority: Pubkey, // 32
        pub withheld_amount: u64, // 8
        pub older_epoch: u64, // 8
        pub older_maximum_fee: u64, // 8
        pub older_transfer_fee_basis_point: u16, // 2
        pub newer_epoch: u64, // 8
        pub newer_maximum_fee: u64, // 8
        pub newer_transfer_fee_basis_point: u16, // 2
    }
    impl MintWithTransferFeeConfigLayout {
        pub const LEN: usize = 82 + 83 + 1 + 2 + 2 + 108;
    }

    /// Maximum possible fee in basis points is 100%, aka 10_000 basis points
    const MAX_FEE_BASIS_POINTS: u16 = 10_000;
    const MAX_FEE: u64 = 1_000_000_000;
    const MAX_AMOUNT: u64 = 0xFFFFFFFF;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]
        #[test]
        fn test_calculate_transfer_fee_included_amount(
            amount in 0..MAX_AMOUNT,
            maximum_fee in 0..MAX_FEE,
            transfer_fee_basis_point in 0..MAX_FEE_BASIS_POINTS
        ) {
            // stub Clock
            solana_program::program_stubs::set_syscall_stubs(Box::new(SyscallStubs {}));
            assert_eq!(Clock::get().unwrap().epoch, 0);

            let mint_with_transfer_fee_config = MintWithTransferFeeConfigLayout {
                is_initialized: true,
                account_type: 1, // Mint
                extension_type: 1, // TransferFeeConfig
                extension_length: 108,
                older_epoch: 0,
                older_maximum_fee: maximum_fee,
                older_transfer_fee_basis_point: transfer_fee_basis_point,
                newer_epoch: 0,
                newer_maximum_fee: maximum_fee,
                newer_transfer_fee_basis_point: transfer_fee_basis_point,
                ..Default::default()
            };

            let mut data = Vec::<u8>::new();
            mint_with_transfer_fee_config.serialize(&mut data).unwrap();
            assert_eq!(data.len(), MintWithTransferFeeConfigLayout::LEN);

            let key = Pubkey::default();
            let mut lamports = 0u64;
            let owner = anchor_spl::token_2022::ID;
            let rent_epoch = 0;
            let is_signer = false;
            let is_writable = false;
            let executable = false;
            let account_info = AccountInfo::new(
                &key,
                is_signer,
                is_writable,
                &mut lamports,
                &mut data,
                &owner,
                executable,
                rent_epoch
            );
    
            let interface_account_mint = InterfaceAccount::<InterfaceMint>::try_from(&account_info).unwrap();

            let transfer_fee = get_epoch_transfer_fee(&interface_account_mint).unwrap().unwrap();
            assert_eq!(u64::from(transfer_fee.maximum_fee), maximum_fee);
            assert_eq!(u16::from(transfer_fee.transfer_fee_basis_points), transfer_fee_basis_point);

            let _ = calculate_transfer_fee_included_amount(&interface_account_mint, amount)?;
        }
    }
}