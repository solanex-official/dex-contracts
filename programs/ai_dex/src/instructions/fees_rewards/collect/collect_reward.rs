use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::memo::Memo;

use crate::util::{parse_remaining_accounts, AccountsType, RemainingAccountsInfo};
use crate::{
    constants::transfer_memo,
    state::*,
    util::{transfer_from_vault_to_owner, verify_position_authority},
    errors::ErrorCode,
};

#[event]
pub struct RewardCollectedEvent {
    pub ai_dex_pool: Pubkey,
    pub position_key: Pubkey,
    pub position_authority: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_vault: Pubkey,
    pub reward_owner_account: Pubkey,
    pub reward_index: u8,
    pub transfer_amount: u64,
    pub updated_amount_owed: u64,
}

#[derive(Accounts)]
#[instruction(reward_index: u8)]
pub struct CollectReward<'info> {
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    pub position_authority: Signer<'info>,

    #[account(mut, has_one = ai_dex_pool)]
    pub position: Box<Account<'info, Position>>,
    #[account(
        constraint = position_token_account.mint == position.position_mint,
        constraint = position_token_account.amount == 1
    )]
    pub position_token_account: Box<Account<'info, token::TokenAccount>>,

    // #[account(mut,
    //     constraint = reward_owner_account.mint == ai_dex_pool.reward_infos[reward_index as usize].mint
    // )]
    #[account(mut)]
    pub reward_owner_account: Box<InterfaceAccount<'info, TokenAccount>>,

    // #[account(address = ai_dex_pool.reward_infos[reward_index as usize].mint)]
    #[account(mut)]
    pub reward_mint: Box<InterfaceAccount<'info, Mint>>,

    // #[account(mut, address = ai_dex_pool.reward_infos[reward_index as usize].vault)]
    #[account(mut)]
    pub reward_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(constraint = reward_token_program.key() == reward_mint.to_account_info().owner.clone())]
    pub reward_token_program: Interface<'info, TokenInterface>,
    pub memo_program: Program<'info, Memo>,

}

/// Collects all harvestable tokens for a specified reward.
///
/// If the AiDex reward vault does not have enough tokens, the maximum number of available
/// tokens will be debited to the user. The unharvested amount remains tracked, and it can be
/// harvested in the future.
///
/// # Parameters
/// - `reward_index` - The reward to harvest. Acceptable values are 0, 1, and 2.
///
/// # Returns
/// - `Ok`: Reward tokens at the specified reward index have been successfully harvested
/// - `Err`: `RewardNotInitializedError` if the specified reward has not been initialized
///          `InvalidRewardIndexError` if the reward index is not 0, 1, or 2
pub fn collect_reward_handler<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CollectReward<'info>>,
    reward_index: u8,
    remaining_accounts_info: Option<RemainingAccountsInfo>,
) -> Result<()> {
    verify_position_authority(
        &ctx.accounts.position_token_account,
        &ctx.accounts.position_authority,
    )?;

    let ai_dex_pool = ctx.accounts.ai_dex_pool.load()?;
    let index = reward_index as usize;

    // Check if the reward index is valid
    if index >= ai_dex_pool.reward_infos.len() {
        return Err(ErrorCode::InvalidRewardIndexError.into());
    }

    let reward_info = &ai_dex_pool.reward_infos[index];
    // Check that the reward_owner_account mint matches the expected mint from reward_infos
    if ctx.accounts.reward_owner_account.mint != reward_info.mint {
        return Err(ErrorCode::InvalidRewardMintError.into());
    }

    // Check that the reward_vault matches the expected vault from reward_infos
    if ctx.accounts.reward_vault.key() != reward_info.vault {
        return Err(ErrorCode::InvalidVault.into());
    }

    // Check that the reward_mint matches the expected mint from reward_infos
    if ctx.accounts.reward_mint.key() != reward_info.mint {
        return Err(ErrorCode::InvalidRewardMintError.into());
    }

    // Process remaining accounts
    let remaining_accounts = parse_remaining_accounts(
        &ctx.remaining_accounts,
        &remaining_accounts_info,
        &[
            AccountsType::TransferHookReward,
        ],
    )?;

    let index = reward_index as usize;

    let position = &mut ctx.accounts.position;
    let (transfer_amount, updated_amount_owed) = calculate_collect_reward(
        position.reward_infos[index],
        ctx.accounts.reward_vault.amount,
    );

    position.update_reward_owed(index, updated_amount_owed);

    transfer_from_vault_to_owner(
        &ctx.accounts.ai_dex_pool,
        &ctx.accounts.reward_mint,
        &ctx.accounts.reward_vault,
        &ctx.accounts.reward_owner_account,
        &ctx.accounts.reward_token_program,
        &ctx.accounts.memo_program,
        &remaining_accounts.transfer_hook_reward,
        transfer_amount,
        transfer_memo::TRANSFER_MEMO_COLLECT_REWARD.as_bytes(),
    )?;

    emit!(RewardCollectedEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        position_key: ctx.accounts.position.key(),
        position_authority: ctx.accounts.position_authority.key(),
        reward_mint: ctx.accounts.reward_mint.key(),
        reward_vault: ctx.accounts.reward_vault.key(),
        reward_owner_account: ctx.accounts.reward_owner_account.key(),
        reward_index,
        transfer_amount,
        updated_amount_owed,
    });    

    Ok(())
}

fn calculate_collect_reward(position_reward: PositionRewardInfo, vault_amount: u64) -> (u64, u64) {
    let amount_owed = position_reward.amount_owed;
    let (transfer_amount, updated_amount_owed) = if amount_owed > vault_amount {
        (vault_amount, amount_owed - vault_amount)
    } else {
        (amount_owed, 0)
    };

    (transfer_amount, updated_amount_owed)
}

#[cfg(test)]
mod unit_tests {
    use super::calculate_collect_reward;
    use crate::state::PositionRewardInfo;

    #[test]
    fn test_calculate_collect_reward_vault_insufficient_tokens() {
        let (transfer_amount, updated_amount_owed) =
            calculate_collect_reward(position_reward(10), 1);

        assert_eq!(transfer_amount, 1);
        assert_eq!(updated_amount_owed, 9);
    }

    #[test]
    fn test_calculate_collect_reward_vault_sufficient_tokens() {
        let (transfer_amount, updated_amount_owed) =
            calculate_collect_reward(position_reward(10), 10);

        assert_eq!(transfer_amount, 10);
        assert_eq!(updated_amount_owed, 0);
    }

    fn position_reward(amount_owed: u64) -> PositionRewardInfo {
        PositionRewardInfo {
            amount_owed,
            ..Default::default()
        }
    }
}
