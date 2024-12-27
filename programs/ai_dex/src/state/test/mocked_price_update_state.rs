// state.rs

use anchor_lang::prelude::*;

#[account]
pub struct MockPriceUpdate {
    pub price: i64,
    pub conf: u64,
    pub exponent: i32,
    pub publish_time: i64,
}

impl MockPriceUpdate {
    pub const LEN: usize = 8 + // discriminator
        8 + // price
        8 + // conf
        4 + // exponent
        8; // publish_time

    pub fn initialize(
        &mut self,
        price: i64,
        conf: u64,
        exponent: i32,
        publish_time: i64,
    ) -> Result<()> {
        self.price = price;
        self.conf = conf;
        self.exponent = exponent;
        self.publish_time = publish_time;
        Ok(())
    }
}
