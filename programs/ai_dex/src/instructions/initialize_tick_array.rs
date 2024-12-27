use anchor_lang::prelude::*;

use crate::state::*;

#[event]
pub struct TickArrayInitializedEvent {
    pub ai_dex_pool: Pubkey,
    pub funder: Pubkey,
    pub tick_array: Pubkey,
    pub start_tick_index: i32,  // Assuming tick indices are 32-bit integers
}

#[derive(Accounts)]
#[instruction(start_tick_index: i32)]
pub struct InitializeTickArray<'info> {
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    #[account(mut)]
    pub funder: Signer<'info>,

    #[account(
        init,
        payer = funder,
        seeds = [b"tick_array", ai_dex_pool.key().as_ref(), start_tick_index.to_string().as_bytes()],
        bump,
        space = TickArray::LEN
    )]
    pub tick_array: AccountLoader<'info, TickArray>,

    pub system_program: Program<'info, System>,
}

/// Initializes a tick array with the given starting tick index.
///
/// # Arguments
///
/// * `ctx` - The context containing the accounts required for initialization.
/// * `start_tick_index` - The starting index for the tick array.
///
/// # Returns
///
/// * `Result<()>` - Returns an Ok result if the initialization is successful, otherwise returns an error.
///
/// # Errors
///
/// This function will return an error if:
/// - The tick array cannot be loaded for initialization.
/// - The tick array initialization fails.
pub fn initialize_tick_array_handler(ctx: Context<InitializeTickArray>, start_tick_index: i32) -> Result<()> {
    // Attempt to load and initialize the tick array
    let mut tick_array = match ctx.accounts.tick_array.load_init() {
        Ok(array) => array,
        Err(e) => {
            msg!("Error: Failed to load tick array for initialization - {:?}", e);
            return Err(e);
        }
    };

    // Attempt to initialize the tick array
    match tick_array.initialize(&ctx.accounts.ai_dex_pool, start_tick_index) {
        Ok(_) => {
            // Emit a log event after successful initialization
            // Structured JSON logging
            emit!(TickArrayInitializedEvent {
                ai_dex_pool: ctx.accounts.ai_dex_pool.key(),
                funder: ctx.accounts.funder.key(),
                tick_array: ctx.accounts.tick_array.key(),
                start_tick_index,
            });
            
            Ok(())
        },
        Err(e) => {
            // Handle initialization error and log the issue
            msg!("Error: Failed to initialize tick array - {:?}", e);
            Err(e)
        }
    }
}