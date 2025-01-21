use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::memo::Memo;
use crate::orchestrator::swap_orchestrator::PostSwapUpdate;
use crate::state::{AiDexConfig, OracleAccount, SwapReferral};
use crate::swap_with_transfer_fee_extension;
use crate::util::{
    calculate_transfer_fee_excluded_amount, parse_remaining_accounts, transfer_referral_fee, update_and_two_hop_swap_ai_dex, AccountsType, RemainingAccountsInfo
};
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
    pub token_owner_account_input: Pubkey,
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
    #[account(mut)]
    pub ai_dex_one: AccountLoader<'info, AiDexPool>,

    #[account(mut)]
    pub ai_dex_two: AccountLoader<'info, AiDexPool>,

    #[account(mut)]
    pub token_mint_input: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    pub token_mint_intermediate: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    pub token_mint_output: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        constraint = token_program_input.key() == token_mint_input.to_account_info().owner.clone()
    )]
    pub token_program_input: Interface<'info, TokenInterface>,
    #[account(
        constraint = token_program_intermediate.key() == token_mint_intermediate.to_account_info().owner.clone()
    )]
    pub token_program_intermediate: Interface<'info, TokenInterface>,
    #[account(
        constraint = token_program_output.key() == token_mint_output.to_account_info().owner.clone()
    )]
    pub token_program_output: Interface<'info, TokenInterface>,

    #[account(mut, constraint = token_owner_account_input.mint == token_mint_input.key())]
    pub token_owner_account_input: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub token_vault_one_input: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub token_vault_one_intermediate: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub token_vault_two_intermediate: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub token_vault_two_output: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, constraint = token_owner_account_output.mint == token_mint_output.key())]
    pub token_owner_account_output: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_authority: Signer<'info>,

    #[account(mut, constraint = tick_array_one_0.load()?.ai_dex_pool == ai_dex_one.key())]
    pub tick_array_one_0: AccountLoader<'info, TickArray>,
    #[account(mut, constraint = tick_array_one_1.load()?.ai_dex_pool == ai_dex_one.key())]
    pub tick_array_one_1: AccountLoader<'info, TickArray>,
    #[account(mut, constraint = tick_array_one_2.load()?.ai_dex_pool == ai_dex_one.key())]
    pub tick_array_one_2: AccountLoader<'info, TickArray>,

    #[account(mut, constraint = tick_array_two_0.load()?.ai_dex_pool == ai_dex_two.key())]
    pub tick_array_two_0: AccountLoader<'info, TickArray>,
    #[account(mut, constraint = tick_array_two_1.load()?.ai_dex_pool == ai_dex_two.key())]
    pub tick_array_two_1: AccountLoader<'info, TickArray>,
    #[account(mut, constraint = tick_array_two_2.load()?.ai_dex_pool == ai_dex_two.key())]
    pub tick_array_two_2: AccountLoader<'info, TickArray>,

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

pub fn two_hop_swap_handler<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, TwoHopSwap<'info>>,
    amount: u64,
    other_amount_threshold: u64,
    amount_specified_is_input: bool,
    a_to_b_one: bool,
    a_to_b_two: bool,
    sqrt_price_limit_one_bytes: [u8; 16],
    sqrt_price_limit_two_bytes: [u8; 16],
    remaining_accounts_info: Option<RemainingAccountsInfo>,
) -> Result<()> {
    let timestamp = to_timestamp_u64(Clock::get()?.unix_timestamp)?;

    let mut ai_dex_one_data = ctx.accounts.ai_dex_one.load_mut()?;
    let mut ai_dex_two_data = ctx.accounts.ai_dex_two.load_mut()?;

    validate_inputs(
        &ctx,
        &mut *ai_dex_one_data,
        &mut *ai_dex_two_data,
        a_to_b_one,
        a_to_b_two
    )?;

    let remaining_accounts = parse_remaining_accounts(
        &ctx.remaining_accounts,
        &remaining_accounts_info,
        &[
            AccountsType::TransferHookInput,
            AccountsType::TransferHookIntermediate,
            AccountsType::TransferHookOutput,
        ],
    )?;

    // Update oracles if needed
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

    // Grab referral fee rates
    let (referrer_swap_fee_rate_one, referrer_swap_fee_rate_two) = compute_referral_fee_rates(
        &ctx.accounts.swap_referral_one,
        &ctx.accounts.swap_referral_two,
        ctx.accounts.ai_dex_config_one.default_swap_referral_reward_fee_rate,
        ctx.accounts.ai_dex_config_two.default_swap_referral_reward_fee_rate,
    );

    // Create tick sequences
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
                u128::from_le_bytes(sqrt_price_limit_one_bytes),
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
                u128::from_le_bytes(sqrt_price_limit_two_bytes),
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
                u128::from_le_bytes(sqrt_price_limit_two_bytes),
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
                u128::from_le_bytes(sqrt_price_limit_one_bytes),
                false,
                a_to_b_one,
                timestamp,
                referrer_swap_fee_rate_one,
            )?;
            (swap_calc_one, swap_calc_two)
        },
    };

    check_swap_mismatch(&swap_update_one, &swap_update_two, a_to_b_one, a_to_b_two)?;

    check_slippage(
        &ctx,
        &swap_update_one,
        &swap_update_two,
        amount_specified_is_input,
        other_amount_threshold,
        a_to_b_one,
        a_to_b_two,
    )?;

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
        sqrt_price_limit_one: u128::from_le_bytes(sqrt_price_limit_one_bytes),
        sqrt_price_limit_two: u128::from_le_bytes(sqrt_price_limit_two_bytes),
        sqrt_price_one: ctx.accounts.ai_dex_one.load()?.sqrt_price,
        sqrt_price_two: ctx.accounts.ai_dex_two.load()?.sqrt_price,
        current_tick_one: ctx.accounts.ai_dex_one.load()?.tick_current_index,
        current_tick_two: ctx.accounts.ai_dex_two.load()?.tick_current_index,
        fee_growth_global_a_one: ctx.accounts.ai_dex_one.load()?.fee_growth_global_a,
        fee_growth_global_b_one: ctx.accounts.ai_dex_one.load()?.fee_growth_global_b,
        fee_growth_global_a_two: ctx.accounts.ai_dex_two.load()?.fee_growth_global_a,
        fee_growth_global_b_two: ctx.accounts.ai_dex_two.load()?.fee_growth_global_b,
        timestamp,
        token_owner_account_input: ctx.accounts.token_owner_account_input.key(),
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

//
// ────────────────────────────────────────────────────────────────────────────────
// HELPER FUNCTIONS
// ────────────────────────────────────────────────────────────────────────────────
//

#[inline(never)]
fn compute_referral_fee_rates<'info>(
    swap_referral_one: &Option<Account<'info, SwapReferral>>,
    swap_referral_two: &Option<Account<'info, SwapReferral>>,
    default_swap_referral_reward_fee_rate_config_one: u16,
    default_swap_referral_reward_fee_rate_config_two: u16,

) -> (u16, u16) {
    let (config_referral_fee_rate_one, referral_account_fee_rate_one) = if let Some(referral_account) = swap_referral_one {
        (
            default_swap_referral_reward_fee_rate_config_one,
            referral_account.referral_reward_fee_rate
        )
    } else {
        (0, 0)
    };

    let (config_referral_fee_rate_two, referral_account_fee_rate_two) = if let Some(referral_account) = swap_referral_two {
        (
            default_swap_referral_reward_fee_rate_config_two,
            referral_account.referral_reward_fee_rate
        )
    } else {
        (0, 0)
    };

    let referrer_swap_fee_rate_one = std::cmp::max(config_referral_fee_rate_one, referral_account_fee_rate_one);
    let referrer_swap_fee_rate_two = std::cmp::max(config_referral_fee_rate_two, referral_account_fee_rate_two);

    (referrer_swap_fee_rate_one, referrer_swap_fee_rate_two)
}

#[inline(never)]
fn check_swap_mismatch(
    swap_update_one: &PostSwapUpdate,
    swap_update_two: &PostSwapUpdate,
    a_to_b_one: bool,
    a_to_b_two: bool,
) -> Result<()> {
    let swap_calc_one_output = if a_to_b_one {
        swap_update_one.amount_b
    } else {
        swap_update_one.amount_a
    };
    let swap_calc_two_input = if a_to_b_two {
        swap_update_two.amount_a
    } else {
        swap_update_two.amount_b
    };

    if swap_calc_one_output != swap_calc_two_input {
        return Err(ErrorCode::AmountMismatchError.into());
    }
    Ok(())
}

#[inline(never)]
fn check_slippage<'info>(
    ctx: &Context<TwoHopSwap<'info>>,
    swap_update_one: &PostSwapUpdate,
    swap_update_two: &PostSwapUpdate,
    amount_specified_is_input: bool,
    other_amount_threshold: u64,
    a_to_b_one: bool,
    a_to_b_two: bool,
) -> Result<()> {
    if amount_specified_is_input {
        let output_amount = {
            if a_to_b_two {
                calculate_transfer_fee_excluded_amount(
                    &ctx.accounts.token_mint_output,
                    swap_update_two.amount_b
                )?.amount
            } else {
                calculate_transfer_fee_excluded_amount(
                    &ctx.accounts.token_mint_output,
                    swap_update_two.amount_a
                )?.amount
            }
        };
        if output_amount < other_amount_threshold {
            return Err(ErrorCode::AmountOutBelowMinimumError.into());
        }
    } else {
        let input_amount = if a_to_b_one {
            swap_update_one.amount_a
        } else {
            swap_update_one.amount_b
        };
        if input_amount > other_amount_threshold {
            return Err(ErrorCode::AmountInAboveMaximumError.into());
        }
    }
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

    if ctx.accounts.ai_dex_config_one.key() != ai_dex_one_data.ai_dex_config
        || ctx.accounts.ai_dex_config_two.key() != ai_dex_two_data.ai_dex_config
    {
        return Err(ErrorCode::InvalidAiDexConfig.into());
    }

    Ok((swap_one_output_mint, swap_two_input_mint))
}
