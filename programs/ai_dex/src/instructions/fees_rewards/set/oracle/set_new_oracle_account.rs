use anchor_lang::prelude::*;

use crate::state::{AiDexPool, AiDexConfig};

#[event]
pub struct NewOracleAccountSetEvent {
    pub ai_dex_config: Pubkey,
    pub ai_dex_pool: Pubkey,
    pub config_authority: Pubkey,
    pub new_oracle_account: Pubkey
}

#[derive(Accounts)]
pub struct SetNewOracleAccount<'info> {
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(mut, has_one = ai_dex_config)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,

    /// CHECK: the account that will be new oracle can be arbitrary
    pub new_oracle_account: UncheckedAccount<'info>,
}

pub fn set_new_oracle_handler(
    ctx: Context<SetNewOracleAccount>,
) -> Result<()> {
    ctx
        .accounts
        .ai_dex_pool
        .load_mut()?
        .update_oracle_account(ctx.accounts.new_oracle_account.key());

    emit!(NewOracleAccountSetEvent {
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
        config_authority: ctx.accounts.config_authority.key(),
        new_oracle_account: ctx.accounts.new_oracle_account.key(),
    });

    Ok(())
}
