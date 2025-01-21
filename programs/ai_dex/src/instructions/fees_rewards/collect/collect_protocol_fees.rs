use crate::util::{parse_remaining_accounts, AccountsType, RemainingAccountsInfo};
use crate::{
    constants::transfer_memo,
    state::*,
    util::transfer_from_vault_to_owner,
    errors::ErrorCode,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::memo::Memo;

#[event]
pub struct CollectProtocolFeesEvent {
    pub ai_dex_pool: Pubkey,
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,
    pub token_mint_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_destination_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub token_destination_b: Pubkey,
}

#[derive(Accounts)]
pub struct CollectProtocolFees<'info> {
    pub ai_dex_config: Box<Account<'info, AiDexConfig>>,

    #[account(mut, has_one = ai_dex_config)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,

    // #[account(address = ai_dex_pool.token_mint_a)]
    #[account(mut)]
    pub token_mint_a: InterfaceAccount<'info, Mint>,
    // #[account(address = ai_dex_pool.token_mint_b)]
    #[account(mut)]
    pub token_mint_b: InterfaceAccount<'info, Mint>,

    // #[account(mut, address = ai_dex_pool.token_vault_a)]
    #[account(mut)]
    pub token_vault_a: InterfaceAccount<'info, TokenAccount>,

    // #[account(mut, address = ai_dex_pool.token_vault_b)]
    #[account(mut)]
    pub token_vault_b: InterfaceAccount<'info, TokenAccount>,

    // #[account(mut, constraint = token_destination_a.mint == ai_dex_pool.token_mint_a)]
    #[account(mut)]
    pub token_destination_a: InterfaceAccount<'info, TokenAccount>,

    // #[account(mut, constraint = token_destination_b.mint == ai_dex_pool.token_mint_b)]
    #[account(mut)]
    pub token_destination_b: InterfaceAccount<'info, TokenAccount>,

    #[account(constraint = token_program_a.key() == token_mint_a.to_account_info().owner.clone())]
    pub token_program_a: Interface<'info, TokenInterface>,

    #[account(constraint = token_program_b.key() == token_mint_b.to_account_info().owner.clone())]
    pub token_program_b: Interface<'info, TokenInterface>,
    pub memo_program: Program<'info, Memo>,

}

/// Handles the collection of protocol fees.
///
/// This function processes any remaining accounts and transfers the owed protocol fees
/// from the vault to the destination accounts.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for the protocol fee collection.
/// * `remaining_accounts_info` - Optional information about remaining accounts.
///
/// # Returns
///
/// * `Result<()>` - Returns an Ok result if the protocol fee collection is successful, otherwise returns an error.
///
/// # Errors
///
/// This function will return an error if:
/// * Parsing the remaining accounts fails.
/// * Transferring protocol fees from the vault to the destination accounts fails.
pub fn collect_protocol_fees_handler<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CollectProtocolFees<'info>>,
    remaining_accounts_info: Option<RemainingAccountsInfo>,
) -> Result<()> {
    let mut ai_dex_pool = ctx.accounts.ai_dex_pool.load_mut()?;

    // Validate mints, vaults, and destination accounts against expected pool values.
    if ctx.accounts.token_mint_a.key() != ai_dex_pool.token_mint_a {
        return Err(ErrorCode::InvalidRewardMintError.into());
    }
    if ctx.accounts.token_mint_b.key() != ai_dex_pool.token_mint_b {
        return Err(ErrorCode::InvalidRewardMintError.into());
    }
    if ctx.accounts.token_vault_a.key() != ai_dex_pool.token_vault_a {
        return Err(ErrorCode::InvalidVault.into());
    }
    if ctx.accounts.token_vault_b.key() != ai_dex_pool.token_vault_b {
        return Err(ErrorCode::InvalidVault.into());
    }
    if ctx.accounts.token_destination_a.mint != ai_dex_pool.token_mint_a {
        return Err(ErrorCode::InvalidTokenOwner.into());
    }
    if ctx.accounts.token_destination_b.mint != ai_dex_pool.token_mint_b {
        return Err(ErrorCode::InvalidTokenOwner.into());
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

    let protocol_fee_owed_a = ai_dex_pool.protocol_fee_owed_a;
    let protocol_fee_owed_b = ai_dex_pool.protocol_fee_owed_b;

    // Reset fees owed before performing transfers
    ai_dex_pool.reset_protocol_fees_owed();
    drop(ai_dex_pool);

    // Transfer the owed protocol fee for Token A if non-zero.
    if protocol_fee_owed_a > 0 {
        transfer_from_vault_to_owner(
            &ctx.accounts.ai_dex_pool,
            &ctx.accounts.token_mint_a,
            &ctx.accounts.token_vault_a,
            &ctx.accounts.token_destination_a,
            &ctx.accounts.token_program_a,
            &ctx.accounts.memo_program,
            &remaining_accounts.transfer_hook_a,
            protocol_fee_owed_a,
            transfer_memo::TRANSFER_MEMO_COLLECT_PROTOCOL_FEES.as_bytes(),
        )?;
    }

    // Transfer the owed protocol fee for Token B if non-zero.
    if protocol_fee_owed_b > 0 {
        transfer_from_vault_to_owner(
            &ctx.accounts.ai_dex_pool,
            &ctx.accounts.token_mint_b,
            &ctx.accounts.token_vault_b,
            &ctx.accounts.token_destination_b,
            &ctx.accounts.token_program_b,
            &ctx.accounts.memo_program,
            &remaining_accounts.transfer_hook_b,
            protocol_fee_owed_b,
            transfer_memo::TRANSFER_MEMO_COLLECT_PROTOCOL_FEES.as_bytes(),
        )?;
    }

    emit!(CollectProtocolFeesEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        protocol_fee_owed_a,
        protocol_fee_owed_b,
        token_mint_a: ctx.accounts.token_mint_a.key(),
        token_vault_a: ctx.accounts.token_vault_a.key(),
        token_destination_a: ctx.accounts.token_destination_a.key(),
        token_mint_b: ctx.accounts.token_mint_b.key(),
        token_vault_b: ctx.accounts.token_vault_b.key(),
        token_destination_b: ctx.accounts.token_destination_b.key(),
    });

    Ok(())
}