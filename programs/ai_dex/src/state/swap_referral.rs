use anchor_lang::prelude::*;

use crate::math::MAX_REFERRAL_REWARD_FEE_RATE;
use crate::errors::ErrorCode;

#[account]
pub struct SwapReferral {
    pub referrer_address: Pubkey, // 32 bytes
    pub referral_reward_fee_rate: u16, // 2 bytes
    pub referral_code: String, // 11 bytes
    pub referral_bump: [u8; 1] // 1 byte
}

impl SwapReferral {
    // Define the length of the account.
    pub const LEN: usize = 8 // discriminator
    + 32 // referrer_address
    + 32 // referred_user_address
    + 2  // referral_reward_fee_rate
    + 11 // referral_code
    + 1; // referral_bump

    /// Returns an array of references to the seeds used for program address generation.
    pub fn seeds(&self) -> [&[u8]; 4] {
        [
            &b"swap-referral"[..],
            self.referrer_address.as_ref(),
            self.referral_code.as_ref(),
            self.referral_bump.as_ref(),
        ]
    }

    pub fn initialize_swap_referral(
        &mut self,
        referral_bump: u8,
        referrer_address: Pubkey,
        referral_code: &String,
    ) -> Result<()> {
        self.referral_bump = [referral_bump];
        self.referrer_address = referrer_address;
        self.referral_reward_fee_rate = 0;
        self.referral_code = referral_code.to_string();
        Ok(())
    }

    pub fn update_swap_reward_fee_rate(
        &mut self,
        swap_referral_reward_fee_rate: u16,
    ) -> Result<()> {
        if swap_referral_reward_fee_rate > MAX_REFERRAL_REWARD_FEE_RATE {
            return Err(ErrorCode::ReferralRewardFeeRateExceededError.into());
        }
        if swap_referral_reward_fee_rate == self.referral_reward_fee_rate {
            return Err(ErrorCode::FeeRateUnchanged.into());
        }
        self.referral_reward_fee_rate = swap_referral_reward_fee_rate;
        Ok(())
    }

}