use anchor_lang::prelude::*;

use crate::state::{AiDexPool, AiDexConfig, NUM_REWARDS};
use crate::errors::ErrorCode::InvalidRewardIndexError;

#[event]
pub struct RewardAuthoritySetEvent {
    pub ai_dex_pool: Pubkey,
    pub reward_index: u8,
    pub old_reward_authority: Pubkey,
    pub new_reward_authority: Pubkey,
    pub config_authority: Pubkey,
}

#[derive(Accounts)]
#[instruction(reward_index: u8)]
pub struct SetRewardAuthorityByConfigAuthority<'info> {
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(mut, has_one = ai_dex_config)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,

    /// CHECK: the account that will be new authority can be arbitrary
    pub new_reward_authority: UncheckedAccount<'info>,
}

/// Sets the AiDex reward authority at the provided `reward_index`.
///
/// This function updates the reward authority for a specific reward index in the AI DEX configuration.
/// Only the current reward emissions super authority has permission to invoke this instruction.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for setting the new reward authority.
/// * `reward_index` - The index of the reward for which the authority is to be updated.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the reward authority is successfully updated,
/// or an `Err` if an error occurs.
pub fn set_reward_authority_by_config_authority_handler(
    ctx: Context<SetRewardAuthorityByConfigAuthority>,
    reward_index: u8
) -> Result<()> {
    if reward_index as usize >= NUM_REWARDS {
        return Err(InvalidRewardIndexError.into());
    }

    let mut ai_dex_pool = ctx.accounts.ai_dex_pool.load_mut()?;

    let old_reward_authority = ai_dex_pool.reward_infos[reward_index as usize].authority;
    
    ai_dex_pool.update_reward_authority(
        reward_index as usize,
        ctx.accounts.new_reward_authority.key(),
    )?;

    emit!(RewardAuthoritySetEvent {
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        reward_index,
        old_reward_authority,
        new_reward_authority: ctx.accounts.new_reward_authority.key(),
        config_authority: ctx.accounts.config_authority.key(),
    });
    
    Ok(())
}
