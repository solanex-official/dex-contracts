// instructions/initialize_mock_price.rs

use anchor_lang::prelude::*;

use crate::state::{AiDexConfig, MockPriceUpdate};

#[derive(Accounts)]
pub struct InitializeMockPrice<'info> {
    pub config: Box<Account<'info, AiDexConfig>>,

    #[account(
        init,
        payer = payer,
        space = MockPriceUpdate::LEN,
    )]
    pub price_update: Account<'info, MockPriceUpdate>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(address = config.config_authority)]
    pub config_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_mock_price_handler(
    ctx: Context<InitializeMockPrice>,
    price: i64,
    conf: u64,
    exponent: i32,
    publish_time: i64,
) -> Result<()> {
    let price_update = &mut ctx.accounts.price_update;

    price_update.initialize(price, conf, exponent, publish_time)?;

    Ok(())
}
