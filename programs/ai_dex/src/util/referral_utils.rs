use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::spl_associated_token_account, 
    memo::Memo
};
use anchor_spl::token_interface::{Mint as InterfaceMint, TokenAccount as InterfaceTokenAccount, TokenInterface};
use crate::{
    constants::transfer_memo,
    state::{AiDexPool, SwapReferral},
    errors::ErrorCode,
};
use super::transfer_from_vault_to_owner;

#[event]
pub struct TransferReferralFeeEvent {
    pub ai_dex_pool: Pubkey,
    pub swap_referral: Pubkey,
    pub amount: u64,
    pub token_mint: Pubkey,
    pub token_vault: Pubkey,
    pub token_program: Pubkey,
    pub a_to_b: bool,
    pub swap_referral_ata: Pubkey,
}

pub fn transfer_referral_fee<'info>(
    swap_referral: &Account<SwapReferral>,
    swap_referral_ata_a: Option<&InterfaceAccount<'info, InterfaceTokenAccount>>,
    swap_referral_ata_b: Option<&InterfaceAccount<'info, InterfaceTokenAccount>>,
    token_mint_a: &InterfaceAccount<'info, InterfaceMint>,
    token_mint_b: &InterfaceAccount<'info, InterfaceMint>,
    token_vault_a: &InterfaceAccount<'info, InterfaceTokenAccount>,
    token_vault_b: &InterfaceAccount<'info, InterfaceTokenAccount>,
    token_program_a: &Interface<'info, TokenInterface>,
    token_program_b: &Interface<'info, TokenInterface>,
    memo_program: &Program<'info, Memo>,
    transfer_hook_account_a: &Option<Vec<AccountInfo<'info>>>,
    transfer_hook_account_b: &Option<Vec<AccountInfo<'info>>>,
    ai_dex_pool: &AccountLoader<'info, AiDexPool>,
    amount: u64,
    a_to_b: bool,
) -> Result<()> {
    // Determine the token mint, vault, and program based on swap direction
    let (
        token_mint,
        token_vault,
        token_program,
        swap_referral_ata,
        transfer_hook_account
    ) = if a_to_b {
        // Swap A to B; fee is in token A
        (
            token_mint_a,
            token_vault_a,
            token_program_a,
            swap_referral_ata_a.ok_or(ErrorCode::MissingSwapReferralAta)?,
            transfer_hook_account_a
        )
    } else {
        // Swap B to A; fee is in token B
        (
            token_mint_b,
            token_vault_b,
            token_program_b,
            swap_referral_ata_b.ok_or(ErrorCode::MissingSwapReferralAta)?,
            transfer_hook_account_b
        )
    };

    // Get the referral PDA from swap_referral.key()
    let referral_pda = swap_referral.key();

    // Derive the expected ATA address
    let expected_ata = spl_associated_token_account::get_associated_token_address(
        &referral_pda,
        &token_mint.key(),
    );

    // Check that the swap_referral_ata is the expected ATA
    if swap_referral_ata.key() != expected_ata {
        return Err(ErrorCode::InvalidSwapReferralAta.into());
    }

    // Now, perform the transfer using your existing function
    transfer_from_vault_to_owner(
        ai_dex_pool,
        token_mint,
        token_vault,
        &swap_referral_ata,
        &token_program,
        memo_program,
        transfer_hook_account,
        amount,
        transfer_memo::TRANSFER_MEMO_SEND_REFERRAL_FEES_TO_PDA_ATA.as_bytes(),
    )?;

    // Emit the event
    emit!(TransferReferralFeeEvent {
        ai_dex_pool: ai_dex_pool.key(),
        swap_referral: swap_referral.key(),
        amount,
        token_mint: token_mint.key(),
        token_vault: token_vault.key(),
        token_program: token_program.key(),
        a_to_b,
        swap_referral_ata: swap_referral_ata.key(),
    });

    Ok(())
}