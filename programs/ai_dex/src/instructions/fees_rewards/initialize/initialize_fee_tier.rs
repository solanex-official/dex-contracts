use crate::state::*;
use anchor_lang::prelude::*;

#[event]
pub struct FeeTierInitializedEvent {
    pub config_key: Pubkey,
    pub fee_tier_key: Pubkey,
    pub funder: Pubkey,
    pub tick_spacing: u16,
    pub default_fee_rate: u16,
}

#[derive(Accounts)]
#[instruction(tick_spacing: u16)]
pub struct InitializeFeeTier<'info> {
    pub config: Box<Account<'info, AiDexConfig>>,

    #[account(init,
        payer = funder,
        seeds = [
            b"fee_tier",
            config.key().as_ref(),
            tick_spacing.to_le_bytes().as_ref()
        ],
        bump,
        space = FeeTier::LEN)]
    pub fee_tier: Account<'info, FeeTier>,

    #[account(mut)]
    pub funder: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Initializes a fee tier with specified tick spacing and default fee rate.
///
/// This function handles the initialization of a fee tier. It sets the tick spacing and the
/// default fee rate for the fee tier based on the provided configuration.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts required for initializing the fee tier.
/// * `tick_spacing` - The spacing between ticks for the fee tier.
/// * `default_fee_rate` - The default fee rate to be set for the fee tier.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the fee tier is successfully initialized,
/// or an `Err` if an error occurs.
pub fn initialize_fee_tier_handler(
    ctx: Context<InitializeFeeTier>,
    tick_spacing: u16,
    default_fee_rate: u16,
) -> Result<()> {
    ctx
        .accounts
        .fee_tier
        .initialize(&ctx.accounts.config, tick_spacing, default_fee_rate)?;

    emit!(FeeTierInitializedEvent {
        config_key: ctx.accounts.config.key(),
        fee_tier_key: ctx.accounts.fee_tier.key(),
        funder: ctx.accounts.funder.key(),
        tick_spacing,
        default_fee_rate,
    });        

    Ok(())
}
