use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::memo::Memo;
use crate::state::{AiDexConfig, OracleAccount, SwapReferral};
use crate::swap_with_transfer_fee_extension;
use crate::util::{calculate_transfer_fee_excluded_amount, parse_remaining_accounts, transfer_referral_fee, update_and_two_hop_swap_ai_dex, AccountsType, RemainingAccountsInfo};
use crate::{
    errors::ErrorCode,
    state::{TickArray, AiDexPool},
    util::{to_timestamp_u64, SwapTickSequence},
    constants::transfer_memo,
};

#[event]
pub struct TwoHopSwapEvent {
    pub ai_dex_one: Pubkey,
    pub ai_dex_two: Pubkey,
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub amount_specified_is_input: bool,
    pub a_to_b_one: bool,
    pub a_to_b_two: bool,
    pub sqrt_price_limit_one: u128,
    pub sqrt_price_limit_two: u128,
    pub sqrt_price_one: u128,
    pub sqrt_price_two: u128,
    pub current_tick_one: i32,
    pub current_tick_two: i32,
    pub fee_growth_global_a_one: u128,
    pub fee_growth_global_b_one: u128,
    pub fee_growth_global_a_two: u128,
    pub fee_growth_global_b_two: u128,
    pub timestamp: u64,
    pub token_mint_input: Pubkey,
    pub token_mint_intermediate: Pubkey,
    pub token_mint_output: Pubkey,
    pub token_program_input: Pubkey,
    pub token_program_intermediate: Pubkey,
    pub token_program_output: Pubkey,
    pub token_owner_account_input: Pubkey,
    pub token_vault_one_input: Pubkey,
    pub token_vault_one_intermediate: Pubkey,
    pub token_vault_two_intermediate: Pubkey,
    pub token_vault_two_output: Pubkey,
    pub token_owner_account_output: Pubkey,
    pub token_authority: Pubkey,
    pub tick_array_one_0: Pubkey,
    pub tick_array_one_1: Pubkey,
    pub tick_array_one_2: Pubkey,
    pub tick_array_two_0: Pubkey,
    pub tick_array_two_1: Pubkey,
    pub tick_array_two_2: Pubkey,
}

#[derive(Accounts)]
#[instruction(
    amount: u64,
    other_amount_threshold: u64,
    amount_specified_is_input: bool,
    a_to_b_one: bool,
    a_to_b_two: bool,
)]
/// Represents a two-hop swap operation involving two different AiDex instances.
pub struct TwoHopSwap<'info> {
    /// The first AiDex instance involved in the swap.
    #[account(mut)]
    pub ai_dex_one: AccountLoader<'info, AiDexPool>,
    
    /// The second AiDex instance involved in the swap.
    #[account(mut)]
    pub ai_dex_two: AccountLoader<'info, AiDexPool>,

    /// The mint account for the input token.
    #[account(mut)]
    pub token_mint_input: Box<InterfaceAccount<'info, Mint>>,

    /// The mint account for the intermediate token.
    #[account(mut)]
    pub token_mint_intermediate: Box<InterfaceAccount<'info, Mint>>,

    /// The mint account for the output token.
    #[account(mut)]
    pub token_mint_output: Box<InterfaceAccount<'info, Mint>>,

    /// The token program for the input token.
    #[account(
        constraint = token_program_input.key() == token_mint_input.to_account_info().owner.clone()
    )]
    pub token_program_input: Interface<'info, TokenInterface>,
    /// The token program for the intermediate token.
    #[account(
        constraint = token_program_intermediate.key() == token_mint_intermediate.to_account_info().owner.clone()
    )]
    pub token_program_intermediate: Interface<'info, TokenInterface>,

    /// The token program for the output token.
    #[account(
        constraint = token_program_output.key() == token_mint_output.to_account_info().owner.clone()
    )]
    pub token_program_output: Interface<'info, TokenInterface>,

    /// The token account of the owner for the input token.
    #[account(mut, constraint = token_owner_account_input.mint == token_mint_input.key())]
    pub token_owner_account_input: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The token vault for the input token in the first AiDex.
    #[account(mut)]
    pub token_vault_one_input: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The token vault for the intermediate token in the first AiDex.
    #[account(mut)]
    pub token_vault_one_intermediate: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The token vault for the intermediate token in the second AiDex.
    #[account(mut)]
    pub token_vault_two_intermediate: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The token vault for the output token in the second AiDex.
    #[account(mut)]
    pub token_vault_two_output: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The token account of the owner for the output token.
    #[account(mut, constraint = token_owner_account_output.mint == token_mint_output.key())]
    pub token_owner_account_output: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The authority that signs the transaction.
    pub token_authority: Signer<'info>,

    /// The first tick array for the first AiDex.
    #[account(mut, constraint = tick_array_one_0.load()?.ai_dex_pool == ai_dex_one.key())]
    pub tick_array_one_0: AccountLoader<'info, TickArray>,

    /// The second tick array for the first AiDex.
    #[account(mut, constraint = tick_array_one_1.load()?.ai_dex_pool == ai_dex_one.key())]
    pub tick_array_one_1: AccountLoader<'info, TickArray>,

    /// The third tick array for the first AiDex.
    #[account(mut, constraint = tick_array_one_2.load()?.ai_dex_pool == ai_dex_one.key())]
    pub tick_array_one_2: AccountLoader<'info, TickArray>,

    /// The first tick array for the second AiDex.
    #[account(mut, constraint = tick_array_two_0.load()?.ai_dex_pool == ai_dex_two.key())]
    pub tick_array_two_0: AccountLoader<'info, TickArray>,

    /// The second tick array for the second AiDex.
    #[account(mut, constraint = tick_array_two_1.load()?.ai_dex_pool == ai_dex_two.key())]
    pub tick_array_two_1: AccountLoader<'info, TickArray>,

    /// The third tick array for the second AiDex.
    #[account(mut, constraint = tick_array_two_2.load()?.ai_dex_pool == ai_dex_two.key())]
    pub tick_array_two_2: AccountLoader<'info, TickArray>,

    /// The memo program.
    pub memo_program: Program<'info, Memo>,

    #[account(
        mut,
        constraint = oracle_account_a.mint_a == token_mint_input.key() && oracle_account_a.mint_b == token_mint_intermediate.key()
    )]
    pub oracle_account_a: Option<Account<'info, OracleAccount>>,

    #[account(
        mut,
        constraint = oracle_account_b.mint_a == token_mint_intermediate.key() && oracle_account_b.mint_b == token_mint_output.key()
    )]
    pub oracle_account_b: Option<Account<'info, OracleAccount>>,

    /// Oracle Price Update Account: Can be either a real PriceUpdateV2 or a MockPriceUpdate
    pub price_update: Option<AccountInfo<'info>>,

    #[account(
        mut,
        constraint = swap_referral_one.referrer_address != token_authority.key()
    )]
    pub swap_referral_one: Option<Account<'info, SwapReferral>>,

    #[account(
        mut,
        constraint = swap_referral_two.referrer_address != token_authority.key()
    )]
    pub swap_referral_two: Option<Account<'info, SwapReferral>>,

    #[account(mut, constraint = swap_referral_ata_input.mint == token_mint_input.key())]
    pub swap_referral_ata_input: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, constraint = swap_referral_ata_intermediate.mint == token_mint_intermediate.key())]
    pub swap_referral_ata_intermediate: Option<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, constraint = swap_referral_ata_output.mint == token_mint_output.key())]
    pub swap_referral_ata_output: Option<InterfaceAccount<'info, TokenAccount>>,

    pub ai_dex_config_one: Account<'info, AiDexConfig>,

    pub ai_dex_config_two: Account<'info, AiDexConfig>,

    // Remaining accounts:
    // - Accounts for transfer hook program of token_mint_input
    // - Accounts for transfer hook program of token_mint_intermediate
    // - Accounts for transfer hook program of token_mint_output
}

/// Handles a two-hop swap operation with specified parameters.
///
/// This function performs a two-hop swap, which involves two separate swap operations
/// between three tokens. It ensures that the intermediary token between the two swaps
/// matches and that the output of the first swap is used as the input for the second swap.
///
/// # Arguments
///
/// * `ctx` - The context containing all the accounts and programs required for the swap.
/// * `amount` - The amount to be swapped.
/// * `other_amount_threshold` - The minimum or maximum amount threshold for the swap.
/// * `amount_specified_is_input` - A boolean indicating if the specified amount is the input amount.
/// * `a_to_b_one` - A boolean indicating the direction of the first swap (A to B if true, B to A if false).
/// * `a_to_b_two` - A boolean indicating the direction of the second swap (A to B if true, B to A if false).
/// * `sqrt_price_limit_one` - The square root price limit for the first swap.
/// * `sqrt_price_limit_two` - The square root price limit for the second swap.
/// * `remaining_accounts_info` - Optional information about remaining accounts.
///
/// # Returns
///
/// This function returns a `Result` which is `Ok` if the swap is successful, or an `Err` if an error occurs.
///
/// # Errors
///
/// This function can return errors in the following cases:
/// * Duplicate two-hop pool error if the same pool is used for both swaps.
/// * Invalid intermediary mint error if the intermediary token does not match.
/// * Amount mismatch error if the output of the first swap does not match the input of the second swap.
/// * Amount out below minimum error if the output amount is less than the specified threshold.
/// * Amount in above maximum error if the input amount is more than the specified threshold.
pub fn two_hop_swap_handler<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, TwoHopSwap<'info>>,
    amount: u64,
    other_amount_threshold: u64,
    amount_specified_is_input: bool,
    a_to_b_one: bool,
    a_to_b_two: bool,
    sqrt_price_limit_one: u128,
    sqrt_price_limit_two: u128,
    remaining_accounts_info: Option<RemainingAccountsInfo>,
) -> Result<()> {
    msg!("sqrt_price_limit_one: {}", sqrt_price_limit_one);
    msg!("sqrt_price_limit_two: {}", sqrt_price_limit_two);
    // Update the global reward growth which increases as a function of time.
    let timestamp = to_timestamp_u64(Clock::get()?.unix_timestamp)?;

    // Load ai_dex_one data
    let mut ai_dex_one_data = ctx.accounts.ai_dex_one.load_mut()?;
    // Load ai_dex_two data
    let mut ai_dex_two_data = ctx.accounts.ai_dex_two.load_mut()?;

    // Validate inputs
    validate_inputs(
        &ctx,
        &mut *ai_dex_one_data,
        &mut *ai_dex_two_data,
        a_to_b_one,
        a_to_b_two
    )?;

    // Process remaining accounts
    let remaining_accounts = parse_remaining_accounts(
        &ctx.remaining_accounts,
        &remaining_accounts_info,
        &[
            AccountsType::TransferHookInput,
            AccountsType::TransferHookIntermediate,
            AccountsType::TransferHookOutput,
        ],
    )?;

    let mut swap_tick_sequence_one = SwapTickSequence::new(
        ctx.accounts.tick_array_one_0.load_mut().unwrap(),
        ctx.accounts.tick_array_one_1.load_mut().ok(),
        ctx.accounts.tick_array_one_2.load_mut().ok(),
    );

    let mut swap_tick_sequence_two = SwapTickSequence::new(
        ctx.accounts.tick_array_two_0.load_mut().unwrap(),
        ctx.accounts.tick_array_two_1.load_mut().ok(),
        ctx.accounts.tick_array_two_2.load_mut().ok(),
    );

    if ai_dex_one_data.is_oracle_pool {
        // Get mutable reference to Account<'info, OracleAccount>
        let oracle_account_a = ctx
            .accounts
            .oracle_account_a
            .as_mut()
            .ok_or(ErrorCode::MissingOracleAccount)?;
        let price_update_account_info = ctx
            .accounts
            .price_update
            .as_ref()
            .ok_or(ErrorCode::MissingPriceUpdate)?;
        oracle_account_a.update_sqrt_price(
            &mut *ai_dex_one_data,
            price_update_account_info,
            ctx.accounts.token_mint_input.decimals,
            ctx.accounts.token_mint_intermediate.decimals,
        )?;
    }

    if ai_dex_two_data.is_oracle_pool {
        // Get mutable reference to Account<'info, OracleAccount>
        let oracle_account_b = ctx
            .accounts
            .oracle_account_b
            .as_mut()
            .ok_or(ErrorCode::MissingOracleAccount)?;
        let price_update_account_info = ctx
            .accounts
            .price_update
            .as_ref()
            .ok_or(ErrorCode::MissingPriceUpdate)?;

        oracle_account_b.update_sqrt_price(
            &mut *ai_dex_two_data, // &mut AiDexPool
            price_update_account_info,
            ctx.accounts.token_mint_intermediate.decimals,
            ctx.accounts.token_mint_output.decimals,
        )?;
    }

    // In the two_hop_swap_handler function:
    let (config_referral_fee_rate_one, referral_account_fee_rate_one) = if let Some(referral_account) = &ctx.accounts.swap_referral_one {
        (
            ctx.accounts.ai_dex_config_one.default_swap_referral_reward_fee_rate,
            referral_account.referral_reward_fee_rate
        )
    } else {
        (0, 0)
    };

    let (config_referral_fee_rate_two, referral_account_fee_rate_two) = if let Some(referral_account) = &ctx.accounts.swap_referral_two {
        (
            ctx.accounts.ai_dex_config_two.default_swap_referral_reward_fee_rate,
            referral_account.referral_reward_fee_rate
        )
    } else {
        (0, 0)
    };

    let referrer_swap_fee_rate_one = std::cmp::max(
        config_referral_fee_rate_one,
        referral_account_fee_rate_one,
    );

    let referrer_swap_fee_rate_two = std::cmp::max(
        config_referral_fee_rate_two,
        referral_account_fee_rate_two,
    );

    // TODO: WLOG, we could extend this to N-swaps, but the account inputs to the instruction would
    // need to be jankier and we may need to programatically map/verify rather than using anchor constraints
    let (swap_update_one, swap_update_two) = match amount_specified_is_input {
        true => {
            // If the amount specified is input, this means we are doing exact-in
            // and the swap calculations occur from Swap 1 => Swap 2
            // and the swaps occur from Swap 1 => Swap 2
            let swap_calc_one = swap_with_transfer_fee_extension(
                &ai_dex_one_data,
                if a_to_b_one { &ctx.accounts.token_mint_input } else { &ctx.accounts.token_mint_intermediate },
                if a_to_b_one { &ctx.accounts.token_mint_intermediate } else { &ctx.accounts.token_mint_input },
                &mut swap_tick_sequence_one,
                amount,
                sqrt_price_limit_one,
                true,
                a_to_b_one,
                timestamp,
                referrer_swap_fee_rate_one,
            )?;
            // Swap two input is the output of swap one
            // We use vault to vault transfer, so transfer fee will be collected once.
            let swap_two_input_amount = match a_to_b_one {
                true => swap_calc_one.amount_b,
                false => swap_calc_one.amount_a,
            };
            let swap_calc_two = swap_with_transfer_fee_extension(
                &ai_dex_two_data,
                if a_to_b_two { &ctx.accounts.token_mint_intermediate } else { &ctx.accounts.token_mint_output },
                if a_to_b_two { &ctx.accounts.token_mint_output } else { &ctx.accounts.token_mint_intermediate },
                &mut swap_tick_sequence_two,
                swap_two_input_amount,
                sqrt_price_limit_two,
                true,
                a_to_b_two,
                timestamp,
                referrer_swap_fee_rate_two,
            )?;
            (swap_calc_one, swap_calc_two)
        },
        false => {
            // If the amount specified is output, this means we need to invert the ordering of the calculations
            // and the swap calculations occur from Swap 2 => Swap 1
            // but the actual swaps occur from Swap 1 => Swap 2 (to ensure that the intermediate token exists in the account)
            let swap_calc_two = swap_with_transfer_fee_extension(
                &ai_dex_two_data,
                if a_to_b_two { &ctx.accounts.token_mint_intermediate } else { &ctx.accounts.token_mint_output },
                if a_to_b_two { &ctx.accounts.token_mint_output } else { &ctx.accounts.token_mint_intermediate },
                &mut swap_tick_sequence_two,
                amount,
                sqrt_price_limit_two,
                false,
                a_to_b_two,
                timestamp,
                referrer_swap_fee_rate_two,
            )?;
            // The output of swap 1 is input of swap_calc_two
            let swap_one_output_amount = match a_to_b_two {
                true => calculate_transfer_fee_excluded_amount(
                    &ctx.accounts.token_mint_intermediate,
                    swap_calc_two.amount_a
                )?.amount,
                false => calculate_transfer_fee_excluded_amount(
                    &ctx.accounts.token_mint_intermediate,
                    swap_calc_two.amount_b
                )?.amount,
            };

            let swap_calc_one = swap_with_transfer_fee_extension(
                &ai_dex_one_data,
                if a_to_b_one { &ctx.accounts.token_mint_input } else { &ctx.accounts.token_mint_intermediate },
                if a_to_b_one { &ctx.accounts.token_mint_intermediate } else { &ctx.accounts.token_mint_input },
                &mut swap_tick_sequence_one,
                swap_one_output_amount,
                sqrt_price_limit_one,
                false,
                a_to_b_one,
                timestamp,
                referrer_swap_fee_rate_one,
            )?;
            (swap_calc_one, swap_calc_two)
        },
    };
    // All output token should be consumed by the second swap
    let swap_calc_one_output = match a_to_b_one {
        true => swap_update_one.amount_b,
        false => swap_update_one.amount_a,
    };
    let swap_calc_two_input = match a_to_b_two {
        true => swap_update_two.amount_a,
        false => swap_update_two.amount_b,
    };

    if swap_calc_one_output != swap_calc_two_input {
        return Err(ErrorCode::AmountMismatchError.into());
    }

    // If amount_specified_is_input == true, then we have a variable amount of output
    // The slippage we care about is the output of the second swap.
    if amount_specified_is_input {
        let output_amount = match a_to_b_two {
            true => calculate_transfer_fee_excluded_amount(
                &ctx.accounts.token_mint_output,
                swap_update_two.amount_b
            )?.amount,
            false => calculate_transfer_fee_excluded_amount(
                &ctx.accounts.token_mint_output,
                swap_update_two.amount_a
            )?.amount,
        };

        // If we have received less than the minimum out, throw an error
        if output_amount < other_amount_threshold {
            return Err(ErrorCode::AmountOutBelowMinimumError.into());
        }
    } else {
        // amount_specified_is_output == false, then we have a variable amount of input
        // The slippage we care about is the input of the first swap
        let input_amount = match a_to_b_one {
            true => swap_update_one.amount_a,
            false => swap_update_one.amount_b,
        };
        if input_amount > other_amount_threshold {
            return Err(ErrorCode::AmountInAboveMaximumError.into());
        }
    }

    drop(ai_dex_one_data);
    drop(ai_dex_two_data);

    // If the first hop produced a referral fee:
    if swap_update_one.next_referral_fee > 0 {
        if let Some(referral_account_one) = &ctx.accounts.swap_referral_one {
            // For the first hop, the referral tokens are likely from input/intermediate tokens
            // Adjust if your logic differs
            transfer_referral_fee(
                referral_account_one,
                ctx.accounts.swap_referral_ata_input.as_ref(),        // Referral ATA for the input token
                ctx.accounts.swap_referral_ata_intermediate.as_ref(), // Referral ATA for the intermediate token
                &ctx.accounts.token_mint_input,
                &ctx.accounts.token_mint_intermediate,
                &*ctx.accounts.token_vault_one_input,
                &*ctx.accounts.token_vault_one_intermediate,
                &ctx.accounts.token_program_input,
                &ctx.accounts.token_program_intermediate,
                &ctx.accounts.memo_program,
                &remaining_accounts.transfer_hook_input,
                &remaining_accounts.transfer_hook_intermediate,
                &mut ctx.accounts.ai_dex_one, // AiDexPool reference from the first hop
                swap_update_one.next_referral_fee,
                a_to_b_one,
            )?;
        } else {
            return Err(ErrorCode::MissingSwapReferralAccount.into());
        }
    }

    // If the second hop produced a referral fee:
    if swap_update_two.next_referral_fee > 0 {
        if let Some(referral_account_two) = &ctx.accounts.swap_referral_two {
            // For the second hop, the referral tokens are likely from intermediate/output tokens
            transfer_referral_fee(
                referral_account_two,
                ctx.accounts.swap_referral_ata_intermediate.as_ref(), // Referral ATA for intermediate token
                ctx.accounts.swap_referral_ata_output.as_ref(),       // Referral ATA for output token
                &ctx.accounts.token_mint_intermediate,
                &ctx.accounts.token_mint_output,
                &*ctx.accounts.token_vault_two_intermediate,
                &*ctx.accounts.token_vault_two_output,
                &ctx.accounts.token_program_intermediate,
                &ctx.accounts.token_program_output,
                &ctx.accounts.memo_program,
                &remaining_accounts.transfer_hook_intermediate,
                &remaining_accounts.transfer_hook_output,
                &mut ctx.accounts.ai_dex_two, // AiDexPool reference from the second hop
                swap_update_two.next_referral_fee,
                a_to_b_two,
            )?;
        } else {
            return Err(ErrorCode::MissingSwapReferralAccount.into());
        }
    }

    update_and_two_hop_swap_ai_dex(
        swap_update_one,
        swap_update_two,
        &mut ctx.accounts.ai_dex_one,
        &mut ctx.accounts.ai_dex_two,
        a_to_b_one,
        a_to_b_two,
        &ctx.accounts.token_mint_input,
        &ctx.accounts.token_mint_intermediate,
        &ctx.accounts.token_mint_output,
        &ctx.accounts.token_program_input,
        &ctx.accounts.token_program_intermediate,
        &ctx.accounts.token_program_output,
        &ctx.accounts.token_owner_account_input,
        &ctx.accounts.token_vault_one_input,
        &ctx.accounts.token_vault_one_intermediate,
        &ctx.accounts.token_vault_two_intermediate,
        &ctx.accounts.token_vault_two_output,
        &ctx.accounts.token_owner_account_output,
        &remaining_accounts.transfer_hook_input,
        &remaining_accounts.transfer_hook_intermediate,
        &remaining_accounts.transfer_hook_output,
        &ctx.accounts.token_authority,
        &ctx.accounts.memo_program,
        timestamp,
        transfer_memo::TRANSFER_MEMO_SWAP.as_bytes(),
    )?;

    emit!(TwoHopSwapEvent {
        ai_dex_one: ctx.accounts.ai_dex_one.key(),
        ai_dex_two: ctx.accounts.ai_dex_two.key(),
        amount,
        other_amount_threshold,
        amount_specified_is_input,
        a_to_b_one,
        a_to_b_two,
        sqrt_price_limit_one,
        sqrt_price_limit_two,
        sqrt_price_one: ctx.accounts.ai_dex_one.load()?.sqrt_price,
        sqrt_price_two: ctx.accounts.ai_dex_two.load()?.sqrt_price,
        current_tick_one: ctx.accounts.ai_dex_one.load()?.tick_current_index,
        current_tick_two: ctx.accounts.ai_dex_two.load()?.tick_current_index,
        fee_growth_global_a_one: ctx.accounts.ai_dex_one.load()?.fee_growth_global_a,
        fee_growth_global_b_one: ctx.accounts.ai_dex_one.load()?.fee_growth_global_b,
        fee_growth_global_a_two: ctx.accounts.ai_dex_two.load()?.fee_growth_global_a,
        fee_growth_global_b_two: ctx.accounts.ai_dex_two.load()?.fee_growth_global_b,
        timestamp,
        token_mint_input: ctx.accounts.token_mint_input.key(),
        token_mint_intermediate: ctx.accounts.token_mint_intermediate.key(),
        token_mint_output: ctx.accounts.token_mint_output.key(),
        token_program_input: ctx.accounts.token_program_input.key(),
        token_program_intermediate: ctx.accounts.token_program_intermediate.key(),
        token_program_output: ctx.accounts.token_program_output.key(),
        token_owner_account_input: ctx.accounts.token_owner_account_input.key(),
        token_vault_one_input: ctx.accounts.token_vault_one_input.key(),
        token_vault_one_intermediate: ctx.accounts.token_vault_one_intermediate.key(),
        token_vault_two_intermediate: ctx.accounts.token_vault_two_intermediate.key(),
        token_vault_two_output: ctx.accounts.token_vault_two_output.key(),
        token_owner_account_output: ctx.accounts.token_owner_account_output.key(),
        token_authority: ctx.accounts.token_authority.key(),
        tick_array_one_0: ctx.accounts.tick_array_one_0.key(),
        tick_array_one_1: ctx.accounts.tick_array_one_1.key(),
        tick_array_one_2: ctx.accounts.tick_array_one_2.key(),
        tick_array_two_0: ctx.accounts.tick_array_two_0.key(),
        tick_array_two_1: ctx.accounts.tick_array_two_1.key(),
        tick_array_two_2: ctx.accounts.tick_array_two_2.key(),
    });

    Ok(())
}

#[inline(never)]
fn validate_inputs(
    ctx: &Context<TwoHopSwap>,
    ai_dex_one_data: &mut AiDexPool,
    ai_dex_two_data: &mut AiDexPool,
    a_to_b_one: bool,
    a_to_b_two: bool,
) -> Result<(Pubkey, Pubkey)> {
    if ctx.accounts.token_mint_input.key() != ai_dex_one_data.input_token_mint(a_to_b_one) {
        return Err(ErrorCode::InvalidInputTokenMint.into());
    }
    if ctx.accounts.token_mint_intermediate.key() != ai_dex_one_data.output_token_mint(a_to_b_one) {
        return Err(ErrorCode::InvalidIntermediateTokenMint.into());
    }
    if ctx.accounts.token_vault_one_input.key() != ai_dex_one_data.input_token_vault(a_to_b_one) {
        return Err(ErrorCode::InvalidVault.into());
    }
    if ctx.accounts.token_vault_one_intermediate.key() != ai_dex_one_data.output_token_vault(a_to_b_one) {
        return Err(ErrorCode::InvalidVault.into());
    }
    let swap_one_output_mint = match a_to_b_one {
        true => ai_dex_one_data.token_mint_b,
        false => ai_dex_one_data.token_mint_a,
    };

    if ctx.accounts.token_mint_output.key() != ai_dex_two_data.output_token_mint(a_to_b_two) {
        return Err(ErrorCode::InvalidOutputTokenMint.into());
    }
    if ctx.accounts.ai_dex_one.key() == ctx.accounts.ai_dex_two.key() {
        return Err(ErrorCode::DuplicateTwoHopPoolError.into());
    }
    if ctx.accounts.token_vault_two_intermediate.key() != ai_dex_two_data.input_token_vault(a_to_b_two) {
        return Err(ErrorCode::InvalidVault.into());
    }
    if ctx.accounts.token_vault_two_output.key() != ai_dex_two_data.output_token_vault(a_to_b_two) {
        return Err(ErrorCode::InvalidVault.into());
    }
    let swap_two_input_mint = match a_to_b_two {
        true => ai_dex_two_data.token_mint_a,
        false => ai_dex_two_data.token_mint_b,
    };

    if swap_one_output_mint != swap_two_input_mint {
        return Err(ErrorCode::InvalidIntermediaryMintError.into());
    }

    if ctx.accounts.ai_dex_config_one.key() != ai_dex_one_data.ai_dex_config ||
    ctx.accounts.ai_dex_config_two.key() != ai_dex_two_data.ai_dex_config {
        return Err(ErrorCode::InvalidAiDexConfig.into());
    }


    Ok((swap_one_output_mint, swap_two_input_mint))
}