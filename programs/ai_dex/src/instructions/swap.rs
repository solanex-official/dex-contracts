use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::memo::Memo;
use crate::state::{AiDexConfig, OracleAccount, SwapReferral};

use crate::util::{
    calculate_transfer_fee_excluded_amount, calculate_transfer_fee_included_amount, parse_remaining_accounts, transfer_referral_fee, AccountsType, RemainingAccountsInfo
};

use crate::{
    errors::ErrorCode,
    orchestrator::swap_orchestrator::*,
    state::{TickArray, AiDexPool},
    util::{to_timestamp_u64, update_and_swap_ai_dex, SwapTickSequence},
    constants::transfer_memo,
};

#[event]
pub struct SwapExecutedEvent {
    pub token_authority: Pubkey,
    pub ai_dex_pool: Pubkey,
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit: u128,
    pub amount_specified_is_input: bool,
    pub a_to_b: bool,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_owner_account_a: Pubkey,
    pub token_owner_account_b: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub tick_array_0: Pubkey,
    pub tick_array_1: Pubkey,
    pub tick_array_2: Pubkey,
    pub sqrt_price: u128,
    pub liquidity: u128,
    pub current_tick: i32,
    pub fee_growth_global_a: u128,
    pub fee_growth_global_b: u128,
    pub timestamp: u64,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    /// The token program for token mint A
    #[account(
        constraint = token_program_a.key() == token_mint_a.to_account_info().owner.clone()
    )]
    pub token_program_a: Interface<'info, TokenInterface>,
    
    /// The token program for token mint B
    #[account(
        constraint = token_program_b.key() == token_mint_b.to_account_info().owner.clone()
    )]
    pub token_program_b: Interface<'info, TokenInterface>,

    /// The memo program
    pub memo_program: Program<'info, Memo>,

    /// The authority that signs transactions
    pub token_authority: Signer<'info>,

    /// The AI DEX account, which is mutable
    #[account(mut, has_one = ai_dex_config)]
    pub ai_dex_pool: AccountLoader<'info, AiDexPool>,

    /// The token mint A account
    // #[account(address = ai_dex_pool.token_mint_a)]
    #[account(mut)]
    pub token_mint_a: InterfaceAccount<'info, Mint>,
    
    /// The token mint B account
    // #[account(address = ai_dex_pool.token_mint_b)]
    #[account(mut)]
    pub token_mint_b: InterfaceAccount<'info, Mint>,
    
    /// The token owner account for token mint A, which is mutable and must match the mint of token mint A
    // #[account(mut, constraint = token_owner_account_a.mint == ai_dex_pool.token_mint_a)]
    #[account(mut)]
    pub token_owner_account_a: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// The token vault account for token mint A, which is mutable and must match the address in the AI DEX
    // #[account(mut, address = ai_dex_pool.token_vault_a)]
    #[account(mut)]
    pub token_vault_a: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The token owner account for token mint B, which is mutable and must match the mint of token mint B
    // #[account(mut, constraint = token_owner_account_b.mint == ai_dex_pool.token_mint_b)]
    #[account(mut)]
    pub token_owner_account_b: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// The token vault account for token mint B, which is mutable and must match the address in the AI DEX
    // #[account(mut, address = ai_dex_pool.token_vault_b)]
    #[account(mut)]
    pub token_vault_b: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The first tick array, which is mutable and must be associated with the AI DEX
    #[account(mut, has_one = ai_dex_pool)]
    pub tick_array_0: AccountLoader<'info, TickArray>,

    /// The second tick array, which is mutable and must be associated with the AI DEX
    #[account(mut, has_one = ai_dex_pool)]
    pub tick_array_1: AccountLoader<'info, TickArray>,

    /// The third tick array, which is mutable and must be associated with the AI DEX
    #[account(mut, has_one = ai_dex_pool)]
    pub tick_array_2: AccountLoader<'info, TickArray>,

    #[account(
        mut,
        constraint = oracle_account.mint_a == token_mint_a.key() && oracle_account.mint_b == token_mint_b.key()
    )]
    pub oracle_account: Option<Account<'info, OracleAccount>>,

    /// Oracle Price Update Account: Can be either a real PriceUpdateV2 or a MockPriceUpdate
    pub price_update: Option<AccountInfo<'info>>,

    #[account(
        mut,
        constraint = swap_referral.referrer_address != token_authority.key()
    )]
    pub swap_referral: Option<Account<'info, SwapReferral>>,

    #[account(mut, constraint = swap_referral_ata_a.mint == token_mint_a.key())]
    pub swap_referral_ata_a: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, constraint = swap_referral_ata_b.mint == token_mint_b.key())]
    pub swap_referral_ata_b: Option<InterfaceAccount<'info, TokenAccount>>,
    
    pub ai_dex_config: Account<'info, AiDexConfig>,
}

pub fn swap_handler<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, Swap<'info>>,
    amount: u64,
    other_amount_threshold: u64,
    sqrt_price_limit: u128,
    amount_specified_is_input: bool,
    a_to_b: bool, // Zero for one
    remaining_accounts_info: Option<RemainingAccountsInfo>,
) -> Result<()> {
    let ai_dex = &mut ctx.accounts.ai_dex_pool;
    let mut ai_dex_data = ai_dex.load_mut()?; // Load ai_dex data once

    // Verify that token_mint_a matches the AiDexPool's token_mint_a
    if ctx.accounts.token_mint_a.key() != ai_dex_data.token_mint_a {
        return Err(ErrorCode::InvalidInputTokenMint.into());
    }

    // Verify that token_mint_b matches the AiDexPool's token_mint_b
    if ctx.accounts.token_mint_b.key() != ai_dex_data.token_mint_b {
        return Err(ErrorCode::InvalidOutputTokenMint.into());
    }

    // Verify that the token owner account for token mint A matches the token_mint_a
    if ctx.accounts.token_owner_account_a.mint != ai_dex_data.token_mint_a {
        return Err(ErrorCode::InvalidTokenOwner.into());
    }

    // Verify that the token owner account for token mint B matches the token_mint_b
    if ctx.accounts.token_owner_account_b.mint != ai_dex_data.token_mint_b {
        return Err(ErrorCode::InvalidTokenOwner.into());
    }

    // Verify that the token vault account for token mint A matches the AiDexPool's token_vault_a
    if ctx.accounts.token_vault_a.key() != ai_dex_data.token_vault_a {
        return Err(ErrorCode::InvalidVault.into());
    }

    // Verify that the token vault account for token mint B matches the AiDexPool's token_vault_b
    if ctx.accounts.token_vault_b.key() != ai_dex_data.token_vault_b {
        return Err(ErrorCode::InvalidVault.into());
    }

    // Update the global reward growth which increases as a function of time.
    let timestamp = to_timestamp_u64(Clock::get()?.unix_timestamp)?;

    // Process remaining accounts
    let remaining_accounts = parse_remaining_accounts(
        &ctx.remaining_accounts,
        &remaining_accounts_info,
        &[
            AccountsType::TransferHookA,
            AccountsType::TransferHookB,
        ],
    )?;

    let mut swap_tick_sequence = SwapTickSequence::new(
        ctx.accounts.tick_array_0.load_mut().unwrap(),
        ctx.accounts.tick_array_1.load_mut().ok(),
        ctx.accounts.tick_array_2.load_mut().ok(),
    );

    if ai_dex_data.is_oracle_pool {
        let oracle_account = ctx
            .accounts
            .oracle_account
            .as_mut()
            .ok_or(ErrorCode::MissingOracleAccount)?;
        let price_update_account_info = ctx
            .accounts
            .price_update
            .as_ref()
            .ok_or(ErrorCode::MissingPriceUpdate)?;

        oracle_account.update_sqrt_price(
            &mut *ai_dex_data,
            price_update_account_info,
            ctx.accounts.token_mint_a.decimals,
            ctx.accounts.token_mint_b.decimals,
        )?;
    }

    let config_referral_reward_fee_rate;
    let referral_account_reward_fee_rate;
    if let Some(referral_account) = &ctx.accounts.swap_referral {
        config_referral_reward_fee_rate = ctx.accounts.ai_dex_config.default_swap_referral_reward_fee_rate;
        referral_account_reward_fee_rate = referral_account.referral_reward_fee_rate;
    } else {
        config_referral_reward_fee_rate = 0;
        referral_account_reward_fee_rate = 0;
    }

    // Compute the referrer swap fee rate as the maximum of the two
    let referrer_swap_fee_rate = std::cmp::max(
        config_referral_reward_fee_rate,
        referral_account_reward_fee_rate,
    );

    let swap_update = swap_with_transfer_fee_extension(
        &ai_dex_data, // Use the already loaded AiDex data
        &ctx.accounts.token_mint_a,
        &ctx.accounts.token_mint_b,
        &mut swap_tick_sequence,
        amount,
        sqrt_price_limit,
        amount_specified_is_input,
        a_to_b,
        timestamp,
        referrer_swap_fee_rate,
    )?;

    drop(ai_dex_data);

    if amount_specified_is_input {
        let transfer_fee_excluded_output_amount = if a_to_b {
            calculate_transfer_fee_excluded_amount(
                &ctx.accounts.token_mint_b,
                swap_update.amount_b
            )?.amount
        } else {
            calculate_transfer_fee_excluded_amount(
                &ctx.accounts.token_mint_a,
                swap_update.amount_a
            )?.amount
        };
        if transfer_fee_excluded_output_amount < other_amount_threshold {
            return Err(ErrorCode::AmountOutBelowMinimumError.into());
        }
    } else {
        let transfer_fee_included_input_amount = if a_to_b {
            swap_update.amount_a
        } else {
            swap_update.amount_b
        };
        if transfer_fee_included_input_amount > other_amount_threshold {
            return Err(ErrorCode::AmountInAboveMaximumError.into());
        }
    }

    if swap_update.next_referral_fee > 0 {
        if let Some(referral_account) = &ctx.accounts.swap_referral {
            transfer_referral_fee(
                referral_account,
                ctx.accounts.swap_referral_ata_a.as_ref(),
                ctx.accounts.swap_referral_ata_b.as_ref(),
                &ctx.accounts.token_mint_a,
                &ctx.accounts.token_mint_b,
                &*ctx.accounts.token_vault_a,
                &*ctx.accounts.token_vault_b,
                &ctx.accounts.token_program_a,
                &ctx.accounts.token_program_b,
                &ctx.accounts.memo_program,
                &remaining_accounts.transfer_hook_a,
                &remaining_accounts.transfer_hook_b,
                &ai_dex,
                swap_update.next_referral_fee,
                a_to_b,
            )?;
        } else {
            return Err(ErrorCode::MissingSwapReferralAccount.into());
        }
    }
    update_and_swap_ai_dex(
        ai_dex,
        &ctx.accounts.token_authority,
        &ctx.accounts.token_mint_a,
        &ctx.accounts.token_mint_b,
        &ctx.accounts.token_owner_account_a,
        &ctx.accounts.token_owner_account_b,
        &ctx.accounts.token_vault_a,
        &ctx.accounts.token_vault_b,
        &remaining_accounts.transfer_hook_a,
        &remaining_accounts.transfer_hook_b,
        &ctx.accounts.token_program_a,
        &ctx.accounts.token_program_b,
        &ctx.accounts.memo_program,
        swap_update,
        a_to_b,
        timestamp,
        transfer_memo::TRANSFER_MEMO_SWAP.as_bytes(),
    )?;

    emit!(SwapExecutedEvent {
        token_authority: ctx.accounts.token_authority.key(),
        ai_dex_pool: ai_dex.key(),
        amount,
        other_amount_threshold,
        sqrt_price_limit,
        amount_specified_is_input,
        a_to_b,
        token_mint_a: ctx.accounts.token_mint_a.key(),
        token_mint_b: ctx.accounts.token_mint_b.key(),
        token_owner_account_a: ctx.accounts.token_owner_account_a.key(),
        token_owner_account_b: ctx.accounts.token_owner_account_b.key(),
        token_vault_a: ctx.accounts.token_vault_a.key(),
        token_vault_b: ctx.accounts.token_vault_b.key(),
        tick_array_0: ctx.accounts.tick_array_0.key(),
        tick_array_1: ctx.accounts.tick_array_1.key(),
        tick_array_2: ctx.accounts.tick_array_2.key(),
        sqrt_price: ai_dex.load()?.sqrt_price,
        liquidity: ai_dex.load()?.liquidity,
        current_tick: ai_dex.load()?.tick_current_index,
        fee_growth_global_a: ai_dex.load()?.fee_growth_global_a,
        fee_growth_global_b: ai_dex.load()?.fee_growth_global_b,
        timestamp,
        token_program_a: ctx.accounts.token_program_a.key(),
        token_program_b: ctx.accounts.token_program_b.key(),
    });

    Ok(())
}

/// Performs a token swap with transfer fee extension.
///
/// # Parameters
/// - `ai_dex`: Reference to the AiDex instance.
/// - `token_mint_a`: Interface account for the first token mint.
/// - `token_mint_b`: Interface account for the second token mint.
/// - `swap_tick_sequence`: Mutable reference to the swap tick sequence.
/// - `amount`: The amount to be swapped.
/// - `sqrt_price_limit`: The square root price limit for the swap.
/// - `amount_specified_is_input`: Boolean indicating if the specified amount is input.
/// - `a_to_b`: Boolean indicating the direction of the swap (true for A to B, false for B to A).
/// - `timestamp`: The timestamp of the swap.
///
/// # Returns
/// - `Result<PostSwapUpdate>`: The result containing the post-swap update or an error.
pub fn swap_with_transfer_fee_extension<'info>(
    ai_dex: &AiDexPool,
    token_mint_a: &InterfaceAccount<'info, Mint>,
    token_mint_b: &InterfaceAccount<'info, Mint>,
    swap_tick_sequence: &mut SwapTickSequence,
    amount: u64,
    sqrt_price_limit: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
    timestamp: u64,
    referrer_swap_fee_rate: u16,
) -> Result<PostSwapUpdate> {
    let (input_token_mint, output_token_mint) = if a_to_b {
        (token_mint_a, token_mint_b)
    } else {
        (token_mint_b, token_mint_a)
    };

    let (transfer_fee_included_amount, transfer_fee_excluded_amount) = if amount_specified_is_input {
        let transfer_fee_excluded_input = calculate_transfer_fee_excluded_amount(input_token_mint, amount)?.amount;
        (amount, transfer_fee_excluded_input)
    } else {
        let transfer_fee_included_output = calculate_transfer_fee_included_amount(output_token_mint, amount)?.amount;
        (transfer_fee_included_output, amount)
    };

    let swap_update = swap(
        ai_dex,
        swap_tick_sequence,
        transfer_fee_excluded_amount,
        sqrt_price_limit,
        amount_specified_is_input,
        a_to_b,
        timestamp,
        referrer_swap_fee_rate,
    )?;

    let (swap_update_amount_input, swap_update_amount_output) = if a_to_b {
        (swap_update.amount_a, swap_update.amount_b)
    } else {
        (swap_update.amount_b, swap_update.amount_a)
    };

    let adjusted_transfer_fee_included_amount = if amount_specified_is_input {
        if swap_update_amount_input == transfer_fee_excluded_amount {
            transfer_fee_included_amount
        } else {
            calculate_transfer_fee_included_amount(input_token_mint, swap_update_amount_input)?.amount
        }
    } else {
        swap_update_amount_output
    };

    let (amount_a, amount_b) = if a_to_b {
        (adjusted_transfer_fee_included_amount, swap_update_amount_output)
    } else {
        (swap_update_amount_output, adjusted_transfer_fee_included_amount)
    };

    Ok(PostSwapUpdate {
        amount_a,
        amount_b,
        next_liquidity: swap_update.next_liquidity,
        next_tick_index: swap_update.next_tick_index,
        next_sqrt_price: swap_update.next_sqrt_price,
        next_fee_growth_global: swap_update.next_fee_growth_global,
        next_reward_infos: swap_update.next_reward_infos,
        next_protocol_fee: swap_update.next_protocol_fee,
        next_referral_fee: swap_update.next_referral_fee,
    })
}