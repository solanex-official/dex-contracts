use crate::constants::transfer_memo;
use crate::util::{parse_remaining_accounts, transfer_from_referral_to_owner, AccountsType, RemainingAccountsInfo};
use crate::{
    state::*,
    errors::ErrorCode,
};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::spl_associated_token_account;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::memo::Memo;

#[event]
pub struct CollectReferralFeesEvent {
    pub swap_referral: Pubkey,
    pub referrer_address: Pubkey,
    pub token_mint: Pubkey,
    pub referral_ata: Pubkey,
    pub destination_token_account: Pubkey,
}

#[derive(Accounts)]
pub struct CollectReferralFees<'info> {
    /// The swap referral account
    #[account(mut, has_one = referrer_address)]
    pub swap_referral: Account<'info, SwapReferral>,

    /// The referrer who signs the transaction
    #[account(signer)]
    pub referrer_address: Signer<'info>,

    /// The token mint of the referral fee
    #[account(mut)]
    pub token_mint: InterfaceAccount<'info, Mint>,

    /// The referral's associated token account (ATA) holding the accumulated fees
    #[account(
        mut, 
        constraint = referral_ata.owner == swap_referral.key(),
        constraint = referral_ata.mint == token_mint.key(),
    )]
    pub referral_ata: InterfaceAccount<'info, TokenAccount>,

    /// The destination token account for the referrer
    #[account(
        mut,
        constraint = destination_token_account.owner == referrer_address.key(),
        constraint = destination_token_account.mint == token_mint.key()
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    /// The token program
    pub token_program: Interface<'info, TokenInterface>,

    /// The memo program
    pub memo_program: Program<'info, Memo>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Rent sysvar
    pub rent: Sysvar<'info, Rent>,
}

pub fn collect_referral_fees_handler<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CollectReferralFees<'info>>,
    amount: u64,
    remaining_accounts_info: Option<RemainingAccountsInfo>,
) -> Result<()> {

    // Verify that the referral ATA is the correct ATA owned by the swap_referral PDA
    let expected_referral_ata = spl_associated_token_account::get_associated_token_address(
        &ctx.accounts.swap_referral.key(),
        &ctx.accounts.token_mint.key(),
    );

    if ctx.accounts.referral_ata.key() != expected_referral_ata {
        return Err(ErrorCode::InvalidSwapReferralAta.into());
    }

    // Process remaining accounts (if any)
    let remaining_accounts = parse_remaining_accounts(
        &ctx.remaining_accounts,
        &remaining_accounts_info,
        &[AccountsType::TransferHookReferralFee],
    )?;

    // Transfer tokens from referral ATA to the referrer's destination token account
    transfer_from_referral_to_owner(
        &ctx.accounts.swap_referral, // Authority: swap_referral PDA
        &ctx.accounts.token_mint,
        &ctx.accounts.referral_ata,
        &ctx.accounts.destination_token_account,
        &ctx.accounts.token_program,
        &ctx.accounts.memo_program,
        &remaining_accounts.transfer_hook_referral_fee,
        amount,
        transfer_memo::TRANSFER_MEMO_COLLECT_REFERRAL_FEES.as_bytes()
    )?;

    emit!(CollectReferralFeesEvent {
        swap_referral: ctx.accounts.swap_referral.key(),
        referrer_address: ctx.accounts.referrer_address.key(),
        token_mint: ctx.accounts.token_mint.key(),
        referral_ata: ctx.accounts.referral_ata.key(),
        destination_token_account: ctx.accounts.destination_token_account.key(),
    });

    Ok(())
}
