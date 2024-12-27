use anchor_lang::prelude::*;
use crate::{errors::ErrorCode, math::MAX_REINVESTMENT_PROTOCOL_FEE_RATE};

#[account]
pub struct AiDexReinvestments {
    pub reinvestments_authority: Pubkey,
    pub default_reinvestment_fee_rate: u16,
}

impl AiDexReinvestments {
    pub const LEN: usize = 8 + 32 + 2;

    pub fn initialize(
        &mut self,
        reinvestments_authority: Pubkey,
        default_reinvestment_fee_rate: u16
    ) -> Result<()> {
        self.reinvestments_authority = reinvestments_authority;
        self.update_default_reinvestment_fee_rate(default_reinvestment_fee_rate)?;
        Ok(())
    }

    pub fn update_default_reinvestment_fee_rate(
        &mut self,
        default_reinvestment_fee_rate: u16,
    ) -> Result<()> {
        if default_reinvestment_fee_rate > MAX_REINVESTMENT_PROTOCOL_FEE_RATE {
            return Err(ErrorCode::ProtocolFeeRateExceededError.into());
        }
        self.default_reinvestment_fee_rate = default_reinvestment_fee_rate;
        Ok(())
    }

    pub fn update_reinvestments_authority(&mut self, reinvestments_authority: Pubkey) -> Result<()> {
        self.reinvestments_authority = reinvestments_authority;
        Ok(())
    }
    
}
