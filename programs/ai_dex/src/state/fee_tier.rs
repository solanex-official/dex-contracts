use crate::state::AiDexConfig;
use crate::{errors::ErrorCode, math::MAX_FEE_RATE};
use anchor_lang::prelude::*;

#[account]
pub struct FeeTier {
    pub ai_dex_config: Pubkey,
    pub tick_spacing: u16,
    pub default_fee_rate: u16,
}

/// Represents a fee tier in the AiDex system.
impl FeeTier {
    /// The length of a fee tier in bytes.
    pub const LEN: usize = 8 + 32 + 4;

    /// Initializes the fee tier with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `ai_dex_config` - The AiDex configuration account.
    /// * `tick_spacing` - The tick spacing value.
    /// * `default_fee_rate` - The default fee rate value.
    ///
    /// # Errors
    ///
    /// Returns an error if the default fee rate exceeds the maximum fee rate.
    pub fn initialize(
        &mut self,
        ai_dex_config: &Account<AiDexConfig>,
        tick_spacing: u16,
        default_fee_rate: u16,
    ) -> Result<()> {
        self.ai_dex_config = ai_dex_config.key();
        self.tick_spacing = tick_spacing;
        self.update_default_fee_rate(default_fee_rate)?;
        Ok(())
    }

    /// Updates the default fee rate of the fee tier.
    ///
    /// # Arguments
    ///
    /// * `default_fee_rate` - The new default fee rate value.
    ///
    /// # Errors
    ///
    /// Returns an error if the default fee rate exceeds the maximum fee rate.
    pub fn update_default_fee_rate(&mut self, default_fee_rate: u16) -> Result<()> {
        if default_fee_rate > MAX_FEE_RATE {
            return Err(ErrorCode::FeeRateExceededError.into());
        }
        self.default_fee_rate = default_fee_rate;

        Ok(())
    }
}
