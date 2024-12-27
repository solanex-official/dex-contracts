use anchor_lang::prelude::*;

use crate::state::*;

#[event]
pub struct ConfigInitializedEvent {
    pub config_key: Pubkey,
    pub super_admin_authority: Pubkey,
    pub config_authority: Pubkey,
    pub default_protocol_fee_rate: u16,
    pub default_referral_reward_fee_rate: u16,
}

#[derive(Accounts)]
#[instruction(config_authority: Pubkey, default_protocol_fee_rate: u16)]
pub struct InitializeConfig<'info> {
    /// SuperAdmin account that stores the current super admin's public key.
    #[account(constraint = super_admin_account.super_admin == super_admin_authority.key())]
    pub super_admin_account: Account<'info, SuperAdmin>,

    /// Signer must be the current super admin.
    #[account(mut)]
    pub super_admin_authority: Signer<'info>,

    #[account(
        init,
        payer = super_admin_authority, space = AiDexConfig::LEN,
        seeds = [
            b"config".as_ref(),
            config_authority.key().as_ref(),
            default_protocol_fee_rate.to_string().as_bytes(),
        ],
        bump,
    )]
    pub config: Account<'info, AiDexConfig>,

    pub system_program: Program<'info, System>,
}

/// Initializes the configuration for the protocol.
///
/// This function handles the initialization of the protocol configuration. It sets up the
/// authorities for fee collection, protocol fee collection, and reward emissions, as well as
/// the default protocol fee rate.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for initializing the configuration.
/// * `config_authority` - The public key of the fee authority.
/// * `default_protocol_fee_rate` - The default protocol fee rate to be set.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the configuration is successfully initialized,
/// or an `Err` if an error occurs.
pub fn initialize_config_handler(
    ctx: Context<InitializeConfig>,
    config_authority: Pubkey,
    default_protocol_fee_rate: u16,
    default_referral_reward_fee_rate: u16,
) -> Result<()> {
    let config = &mut ctx.accounts.config;

    config.initialize(
        config_authority,
        default_protocol_fee_rate,
        default_referral_reward_fee_rate,
    )?;

    emit!(ConfigInitializedEvent {
        config_key: config.key(),
        super_admin_authority: ctx.accounts.super_admin_authority.key(),
        config_authority,
        default_protocol_fee_rate,
        default_referral_reward_fee_rate,
    });
    
    Ok(())
}
