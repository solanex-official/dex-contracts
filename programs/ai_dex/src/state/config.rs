use anchor_lang::prelude::*;

use crate::{errors::ErrorCode, math::{MAX_PROTOCOL_FEE_RATE, MAX_REFERRAL_REWARD_FEE_RATE}};

#[account]
pub struct AiDexConfig {
    pub config_authority: Pubkey,
    pub default_protocol_fee_rate: u16,
    pub default_swap_referral_reward_fee_rate: u16,
}

/// Implementation of the AiDexConfig struct.
impl AiDexConfig {
    /// Length of the AiDexConfig struct.
    pub const LEN: usize = 8 + 32 + 2 + 2;

    /// Updates the fee authority.
    ///
    /// # Arguments
    ///
    /// * `config_authority` - The new config authority public key.
    pub fn update_config_authority(&mut self, config_authority: Pubkey) {
        self.config_authority = config_authority;
    }

    /// Initializes the AiDexConfig struct.
    ///
    /// # Arguments
    ///
    /// * `config_authority` - The fee authority public key.
    /// * `default_protocol_fee_rate` - The default protocol fee rate.
    ///
    /// # Errors
    ///
    /// Returns an error if the default protocol fee rate exceeds the maximum protocol fee rate.
    pub fn initialize(
        &mut self,
        config_authority: Pubkey,
        default_protocol_fee_rate: u16,
        default_swap_referral_reward_fee_rate: u16,
    ) -> Result<()> {
        self.config_authority = config_authority;
        self.update_default_protocol_fee_rate(default_protocol_fee_rate)?;
        self.update_default_swap_referral_reward_fee_rate(default_swap_referral_reward_fee_rate)?;
        Ok(())
    }

    /// Updates the default protocol fee rate.
    ///
    /// # Arguments
    ///
    /// * `default_protocol_fee_rate` - The new default protocol fee rate.
    ///
    /// # Errors
    ///
    /// Returns an error if the default protocol fee rate exceeds the maximum protocol fee rate.
    /// Returns an error if the default protocol fee rate is unchanged.
    pub fn update_default_protocol_fee_rate(
        &mut self,
        default_protocol_fee_rate: u16,
    ) -> Result<()> {
        if default_protocol_fee_rate > MAX_PROTOCOL_FEE_RATE {
            return Err(ErrorCode::ProtocolFeeRateExceededError.into());
        }
        if default_protocol_fee_rate == self.default_protocol_fee_rate {
            return Err(ErrorCode::FeeRateUnchanged.into());
        }
        self.default_protocol_fee_rate = default_protocol_fee_rate;

        Ok(())
    }

    pub fn update_default_swap_referral_reward_fee_rate(
        &mut self,
        default_swap_referral_reward_fee_rate: u16,
    ) -> Result<()> {
        if default_swap_referral_reward_fee_rate > MAX_REFERRAL_REWARD_FEE_RATE {
            return Err(ErrorCode::ReferralRewardFeeRateExceededError.into());
        }
        if default_swap_referral_reward_fee_rate == self.default_swap_referral_reward_fee_rate {
            return Err(ErrorCode::FeeRateUnchanged.into());
        }
        self.default_swap_referral_reward_fee_rate = default_swap_referral_reward_fee_rate;
        Ok(())
    }

}
