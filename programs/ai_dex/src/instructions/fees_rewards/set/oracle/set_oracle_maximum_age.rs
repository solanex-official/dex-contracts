use anchor_lang::prelude::*;

use crate::state::{AiDexConfig, OracleAccount};

#[event]
pub struct NewOracleMaxAgeSetEvent {
    pub ai_dex_config: Pubkey,
    pub oracle_account: Pubkey,
    pub config_authority: Pubkey,
    pub old_maximum_age: u64,
    pub new_maximum_age: u64,
}

#[derive(Accounts)]
pub struct SetNewOracleMaxAgeAccount<'info> {
    pub ai_dex_config: Account<'info, AiDexConfig>,

    #[account(mut)]
    pub oracle_account: Account<'info, OracleAccount>,

    #[account(address = ai_dex_config.config_authority)]
    pub config_authority: Signer<'info>,
}

pub fn set_new_oracle_max_age_handler(
    ctx: Context<SetNewOracleMaxAgeAccount>,
    new_maximum_age: u64
) -> Result<()> {
    let old_maximum_age = ctx.accounts.oracle_account.maximum_age;
    
    ctx
        .accounts
        .oracle_account
        .change_maximum_age(new_maximum_age)?;
    
    emit!(NewOracleMaxAgeSetEvent {
        ai_dex_config: ctx.accounts.ai_dex_config.key(),
        oracle_account: ctx.accounts.oracle_account.key(),
        config_authority: ctx.accounts.config_authority.key(),
        old_maximum_age,
        new_maximum_age,
    });

    Ok(())
}
