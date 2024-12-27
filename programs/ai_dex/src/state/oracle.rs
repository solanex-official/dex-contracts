use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, Price, PriceUpdateV2};
use crate::{
    errors::ErrorCode, math::calculate_initial_sqrt_price, state::MockPriceUpdate
};
use super::AiDexPool;

#[account]
pub struct OracleAccount {
    // Hash ID of the specific token pair's price feed
    pub price_feed_id: String,
    pub maximum_age: u64,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
}

impl OracleAccount {
    // Length of the OracleAccount struct
    pub const LEN: usize = 70  // price_feed_id
        + 8 // discriminator
        + 8 // maximum_age
        + 32 // mint_a
        + 32; // mint_b
    
    pub fn initialize(
        &mut self,
        price_feed_id: String,
        maximum_age: u64,
        mint_a: Pubkey,
        mint_b: Pubkey,
    ) -> Result<()> {
        if mint_a.ge(&mint_b) {
            return Err(ErrorCode::InvalidTokenMintOrderError.into());
        }
        self.price_feed_id = price_feed_id;
        self.maximum_age = maximum_age;
        self.mint_a = mint_a;
        self.mint_b = mint_b;
        Ok(())
    }

    pub fn get_new_sqrt_price(
        &mut self,
        price_update_account_info: &AccountInfo,
        token_decimals_a: u8,
        token_decimals_b: u8,
    ) -> Result<u128> {
        let feed_id: [u8; 32] = get_feed_id_from_hex(&self.price_feed_id)?;
    
        // Determine which account type we're dealing with based on the owner
        let price_data = if price_update_account_info.owner == &pyth_solana_receiver_sdk::ID {
            // Deserialize as PriceUpdateV2
            let mut data = &price_update_account_info.data.borrow()[..];
            let price_update = PriceUpdateV2::try_deserialize(&mut data)
                .map_err(|_| ErrorCode::InvalidPriceUpdateAccount)?;
            price_update.get_price_no_older_than(
                &Clock::get()?,
                self.maximum_age,
                &feed_id,
            )?
        } else if price_update_account_info.owner == &crate::ID {
            // Deserialize as MockPriceUpdate
            let mut data = &price_update_account_info.data.borrow()[..];
            let mock_price_update = MockPriceUpdate::try_deserialize(&mut data)
                .map_err(|_| ErrorCode::InvalidPriceUpdateAccount)?;
            // Construct the Price struct
            Price {
                price: mock_price_update.price,
                conf: mock_price_update.conf,
                exponent: mock_price_update.exponent,
                publish_time: mock_price_update.publish_time,
            }
        } else {
            // Invalid owner
            return Err(ErrorCode::InvalidPriceUpdateAccount.into());
        };
    
        msg!(
            "The price is ({} ± {}) * 10^{}",
            price_data.price,
            price_data.conf,
            price_data.exponent
        );
    
        Ok(calculate_initial_sqrt_price(
            &price_data,
            token_decimals_a,
            token_decimals_b,
        )?)
    }

    pub fn update_sqrt_price(
        &mut self,
        ai_dex: &mut AiDexPool,
        price_update_account_info: &AccountInfo,
        token_decimals_a: u8,
        token_decimals_b: u8,
    ) -> Result<()> {
        let new_sqrt_price = self.get_new_sqrt_price(
            price_update_account_info,
            token_decimals_a,
            token_decimals_b,
        )?;
        ai_dex.update_sqrt_price(new_sqrt_price);
        ai_dex.update_tick_current_index_by_sqrt_price(new_sqrt_price);
        Ok(())
    }

    pub fn change_maximum_age(&mut self, new_maximum_age: u64) -> Result<()> {
        self.maximum_age = new_maximum_age;
        Ok(())
    }

}
