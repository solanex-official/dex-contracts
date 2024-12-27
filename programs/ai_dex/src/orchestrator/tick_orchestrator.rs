use crate::{
    errors::ErrorCode,
    math::add_liquidity_delta,
    state::{Tick, TickUpdate, AiDexRewardInfo, NUM_REWARDS},
};

/// Updates the tick state when crossing a tick boundary.
///
/// This function calculates the new state of a tick when crossing its boundary,
/// updating the fee growth and reward growth outside the tick.
///
/// # Parameters
///
/// - `tick`: A reference to the current `Tick` object.
/// - `fee_growth_global_a`: The global fee growth for token A.
/// - `fee_growth_global_b`: The global fee growth for token B.
/// - `reward_infos`: An array of `AiDexRewardInfo` objects representing the reward information.
///
/// # Returns
///
/// A `Result` containing:
/// - `Ok(TickUpdate)`: The updated tick state.
/// - `Err(ErrorCode)`: An error code if the update fails.
///
/// # Errors
///
/// This function will return an error if any of the calculations for the fee growth or reward growth overflow.
///
/// /// # Notes
///
/// - The function uses wrapping subtraction to handle potential overflow in fee and reward growth calculations.
/// - Only initialized rewards are considered for updating the reward growths outside the tick.
///
pub fn next_tick_cross_update(
    tick: &Tick,
    fee_growth_global_a: u128,
    fee_growth_global_b: u128,
    reward_infos: &[AiDexRewardInfo; NUM_REWARDS],
) -> Result<TickUpdate, ErrorCode> {
    let mut update = TickUpdate::from(tick);

    // Calculate the fee growth outside the tick for token A
    update.fee_growth_outside_a = fee_growth_global_a.wrapping_sub(tick.fee_growth_outside_a);

    // Calculate the fee growth outside the tick for token B
    update.fee_growth_outside_b = fee_growth_global_b.wrapping_sub(tick.fee_growth_outside_b);

    // Calculate the reward growths outside the tick for each reward
    for i in 0..NUM_REWARDS {
        if reward_infos[i].initialized() {
            update.reward_growths_outside[i] = reward_infos[i]
                .growth_global_x64
                .wrapping_sub(tick.reward_growths_outside[i]);
        }
    }

    Ok(update)
}

/// Updates the tick state when modifying liquidity.
///
/// This function calculates the new state of a tick when modifying its liquidity,
/// updating the liquidity, fee growth, and reward growth outside the tick.
///
/// # Parameters
///
/// - `tick`: A reference to the current `Tick` object.
/// - `tick_index`: The index of the current tick.
/// - `tick_current_index`: The index of the current tick being processed.
/// - `fee_growth_global_a`: The global fee growth for token A.
/// - `fee_growth_global_b`: The global fee growth for token B.
/// - `reward_infos`: An array of `AiDexRewardInfo` objects representing the reward information.
/// - `liquidity_delta`: The change in liquidity.
/// - `is_upper_tick`: A flag indicating if the tick being modified is an upper tick.
///
/// # Returns
///
/// A `Result` containing:
/// - `Ok(TickUpdate)`: The updated tick state.
/// - `Err(ErrorCode)`: An error code if the update fails.
///
/// # Notes
///
/// - This function returns a no-op if there is no change in liquidity.
/// - If the remaining liquidity is being removed, the function returns an uninitialized tick.
/// - The function handles the calculation of liquidity gross, fee growth outside the tick,
///   reward growth outside the tick, and liquidity net based on the provided parameters.
pub fn next_tick_modify_liquidity_update(
    tick: &Tick,
    tick_index: i32,
    tick_current_index: i32,
    fee_growth_global_a: u128,
    fee_growth_global_b: u128,
    reward_infos: &[AiDexRewardInfo; NUM_REWARDS],
    liquidity_delta: i128,
    is_upper_tick: bool,
) -> Result<TickUpdate, ErrorCode> {
    // Return a no-op if there is no change in liquidity
    if liquidity_delta == 0 {
        return Ok(TickUpdate::from(tick));
    }

    // Calculate the new liquidity gross
    let liquidity_gross = add_liquidity_delta(tick.liquidity_gross, liquidity_delta)?;

    // Update to an uninitialized tick if remaining liquidity is being removed
    if liquidity_gross == 0 {
        return Ok(TickUpdate::default());
    }

    // Calculate the fee growth and reward growth outside the tick
    let (fee_growth_outside_a, fee_growth_outside_b, reward_growths_outside) =
        calculate_growths(tick, tick_index, tick_current_index, fee_growth_global_a, fee_growth_global_b, reward_infos);

    // Calculate the new liquidity net
    let liquidity_net = calculate_liquidity_net(tick.liquidity_net, liquidity_delta, is_upper_tick)?;

    // Create and return the updated tick state
    Ok(TickUpdate {
        initialized: true,
        liquidity_net,
        liquidity_gross,
        fee_growth_outside_a,
        fee_growth_outside_b,
        reward_growths_outside,
    })
}

/// Calculates the fee growth and reward growth outside the tick based on the tick state and provided parameters.
///
/// # Parameters
///
/// - `tick`: A reference to the current `Tick` object.
/// - `tick_index`: The index of the current tick.
/// - `tick_current_index`: The index of the current tick being processed.
/// - `fee_growth_global_a`: The global fee growth for token A.
/// - `fee_growth_global_b`: The global fee growth for token B.
/// - `reward_infos`: An array of `AiDexRewardInfo` objects representing the reward information.
///
/// # Returns
///
/// A tuple containing:
/// - `u128`: The fee growth outside the tick for token A.
/// - `u128`: The fee growth outside the tick for token B.
/// - `[u128; NUM_REWARDS]`: The reward growths outside the tick for each reward.
///
/// # Notes
///
/// - If the liquidity gross of the tick is zero, the function returns the global fee growth and reward growths.
/// - If the liquidity gross of the tick is non-zero, the function returns the fee growth and reward growths outside the tick.
fn calculate_growths(
    tick: &Tick,
    tick_index: i32,
    tick_current_index: i32,
    fee_growth_global_a: u128,
    fee_growth_global_b: u128,
    reward_infos: &[AiDexRewardInfo; NUM_REWARDS],
) -> (u128, u128, [u128; NUM_REWARDS]) {
    if tick.liquidity_gross == 0 {
        // By convention, assume all prior growth happened below the tick
        if tick_current_index >= tick_index {
            (
                fee_growth_global_a,
                fee_growth_global_b,
                AiDexRewardInfo::to_reward_growths(reward_infos),
            )
        } else {
            (0, 0, [0; NUM_REWARDS])
        }
    } else {
        (
            tick.fee_growth_outside_a,
            tick.fee_growth_outside_b,
            tick.reward_growths_outside,
        )
    }
}

/// Calculates the liquidity net based on the current liquidity net, liquidity delta, and tick position.
///
/// # Parameters
///
/// - `liquidity_net`: The current liquidity net.
/// - `liquidity_delta`: The change in liquidity.
/// - `is_upper_tick`: A flag indicating if the tick being modified is an upper tick.
///
/// # Returns   
///
/// A `Result` containing:
/// - `Ok(i128)`: The updated liquidity net.
/// - `Err(ErrorCode)`: An error code if the calculation fails.
///
/// # Notes
///
/// - If `is_upper_tick` is `true`, the function subtracts `liquidity_delta` from `liquidity_net`.
/// - If `is_upper_tick` is `false`, the function adds `liquidity_delta` to `liquidity_net`.
/// - The function checks for overflow and returns an error if the calculation fails.
fn calculate_liquidity_net(liquidity_net: i128, liquidity_delta: i128, is_upper_tick: bool) -> Result<i128, ErrorCode> {
    if is_upper_tick {
        liquidity_net.checked_sub(liquidity_delta).ok_or(ErrorCode::TickLiquidityNetError)
    } else {
        liquidity_net.checked_add(liquidity_delta).ok_or(ErrorCode::TickLiquidityNetError)
    }
}

// Calculates the fee growths inside of tick_lower and tick_upper based on their
// index relative to tick_current_index.
///
/// # Parameters
///
/// - `tick_current_index`: The index of the current tick being processed.
/// - `tick_lower`: A reference to the lower tick.
/// - `tick_lower_index`: The index of the lower tick.
/// - `tick_upper`: A reference to the upper tick.
/// - `tick_upper_index`: The index of the upper tick.
/// - `fee_growth_global_a`: The global fee growth for token A.
/// - `fee_growth_global_b`: The global fee growth for token B.
///
/// # Returns
///
/// A tuple containing:
/// - `u128`: The fee growth inside the lower tick for token A.
/// - `u128`: The fee growth inside the lower tick for token B.
pub fn next_fee_growths_inside(
    tick_current_index: i32,
    tick_lower: &Tick,
    tick_lower_index: i32,
    tick_upper: &Tick,
    tick_upper_index: i32,
    fee_growth_global_a: u128,
    fee_growth_global_b: u128,
) -> (u128, u128) {
    // By convention, when initializing a tick, all fees have been earned below the tick.
    let (fee_growth_below_a, fee_growth_below_b) = match (tick_lower.initialized, tick_current_index < tick_lower_index) {
        (false, _) => (fee_growth_global_a, fee_growth_global_b),
        (true, true) => (
            fee_growth_global_a.wrapping_sub(tick_lower.fee_growth_outside_a),
            fee_growth_global_b.wrapping_sub(tick_lower.fee_growth_outside_b),
        ),
        (true, false) => (
            tick_lower.fee_growth_outside_a,
            tick_lower.fee_growth_outside_b,
        ),
    };

    // By convention, when initializing a tick, no fees have been earned above the tick.
    let (fee_growth_above_a, fee_growth_above_b) = match (tick_upper.initialized, tick_current_index < tick_upper_index) {
        (false, _) => (0, 0),
        (true, true) => (
            tick_upper.fee_growth_outside_a,
            tick_upper.fee_growth_outside_b,
        ),
        (true, false) => (
            fee_growth_global_a.wrapping_sub(tick_upper.fee_growth_outside_a),
            fee_growth_global_b.wrapping_sub(tick_upper.fee_growth_outside_b),
        ),
    };

    (
        fee_growth_global_a
            .wrapping_sub(fee_growth_below_a)
            .wrapping_sub(fee_growth_above_a),
        fee_growth_global_b
            .wrapping_sub(fee_growth_below_b)
            .wrapping_sub(fee_growth_above_b),
    )
}

/// Calculates the reward growths inside of tick_lower and tick_upper based on their positions
/// relative to tick_current_index. An uninitialized reward will always have a reward growth of zero.
///
/// # Parameters
///
/// - `tick_current_index`: The index of the current tick being processed.
/// - `tick_lower`: A reference to the lower tick.
/// - `tick_lower_index`: The index of the lower tick.
/// - `tick_upper`: A reference to the upper tick.
/// - `tick_upper_index`: The index of the upper tick.
/// - `reward_infos`: An array of `AiDexRewardInfo` objects representing the reward information.
///
/// # Returns
///
/// An array of `u128` values representing the reward growths inside the ticks.
///
/// # Notes
///
/// - An uninitialized reward will always have a reward growth of zero.
pub fn next_reward_growths_inside(
    tick_current_index: i32,
    tick_lower: &Tick,
    tick_lower_index: i32,
    tick_upper: &Tick,
    tick_upper_index: i32,
    reward_infos: &[AiDexRewardInfo; NUM_REWARDS],
) -> [u128; NUM_REWARDS] {
    let mut reward_growths_inside = [0; NUM_REWARDS];

    for i in 0..NUM_REWARDS {
        if !reward_infos[i].initialized() {
            continue;
        }

        // By convention, assume all prior growth happened below the tick
        let reward_growths_below = match (tick_lower.initialized, tick_current_index < tick_lower_index) {
            (false, _) => reward_infos[i].growth_global_x64,
            (true, true) => reward_infos[i]
                .growth_global_x64
                .wrapping_sub(tick_lower.reward_growths_outside[i]),
            (true, false) => tick_lower.reward_growths_outside[i],
        };

        // By convention, assume all prior growth happened below the tick, not above
        let reward_growths_above = match (tick_upper.initialized, tick_current_index < tick_upper_index) {
            (false, _) => 0,
            (true, true) => tick_upper.reward_growths_outside[i],
            (true, false) => reward_infos[i]
                .growth_global_x64
                .wrapping_sub(tick_upper.reward_growths_outside[i]),
        };

        reward_growths_inside[i] = reward_infos[i]
            .growth_global_x64
            .wrapping_sub(reward_growths_below)
            .wrapping_sub(reward_growths_above);
    }

    reward_growths_inside
}

#[cfg(test)]
mod tick_orchestrator_tests {
    use anchor_lang::prelude::Pubkey;

    use crate::{
        errors::ErrorCode,
        orchestrator::tick_orchestrator::{
            next_fee_growths_inside, next_tick_cross_update, next_tick_modify_liquidity_update,
            TickUpdate,
        },
        math::Q64_RESOLUTION,
        state::{tick_builder::TickBuilder, Tick, AiDexRewardInfo, NUM_REWARDS},
    };

    use super::next_reward_growths_inside;

    fn create_test_ai_dex_reward_info(
        emissions_per_second_x64: u128,
        growth_global_x64: u128,
        initialized: bool,
    ) -> AiDexRewardInfo {
        AiDexRewardInfo {
            mint: if initialized {
                Pubkey::new_unique()
            } else {
                Pubkey::default()
            },
            emissions_per_second_x64,
            growth_global_x64,
            ..Default::default()
        }
    }

    #[test]
    fn test_next_fee_growths_inside() {
        struct Test<'a> {
            name: &'a str,
            tick_current_index: i32,
            tick_lower: Tick,
            tick_lower_index: i32,
            tick_upper: Tick,
            tick_upper_index: i32,
            fee_growth_global_a: u128,
            fee_growth_global_b: u128,
            expected_fee_growths_inside: (u128, u128),
        }

        for test in [
            Test {
                name: "current tick index below ticks",
                tick_current_index: -200,
                tick_lower: Tick {
                    initialized: true,
                    fee_growth_outside_a: 2000,
                    fee_growth_outside_b: 1000,
                    ..Default::default()
                },
                tick_lower_index: -100,
                tick_upper: Tick {
                    initialized: true,
                    fee_growth_outside_a: 1000,
                    fee_growth_outside_b: 1000,
                    ..Default::default()
                },
                tick_upper_index: 100,
                fee_growth_global_a: 3000,
                fee_growth_global_b: 3000,
                expected_fee_growths_inside: (1000, 0),
            },
            Test {
                name: "current tick index between ticks",
                tick_current_index: -20,
                tick_lower: Tick {
                    initialized: true,
                    fee_growth_outside_a: 2000,
                    fee_growth_outside_b: 1000,
                    ..Default::default()
                },
                tick_lower_index: -20,
                tick_upper: Tick {
                    initialized: true,
                    fee_growth_outside_a: 1500,
                    fee_growth_outside_b: 1000,
                    ..Default::default()
                },
                tick_upper_index: 100,
                fee_growth_global_a: 4000,
                fee_growth_global_b: 3000,
                expected_fee_growths_inside: (500, 1000),
            },
            Test {
                name: "current tick index above ticks",
                tick_current_index: 200,
                tick_lower: Tick {
                    initialized: true,
                    fee_growth_outside_a: 2000,
                    fee_growth_outside_b: 1000,
                    ..Default::default()
                },
                tick_lower_index: -100,
                tick_upper: Tick {
                    initialized: true,
                    fee_growth_outside_a: 2500,
                    fee_growth_outside_b: 2000,
                    ..Default::default()
                },
                tick_upper_index: 100,
                fee_growth_global_a: 3000,
                fee_growth_global_b: 3000,
                expected_fee_growths_inside: (500, 1000),
            },
        ] {
            // System under test
            let (fee_growth_inside_a, fee_growth_inside_b) = next_fee_growths_inside(
                test.tick_current_index,
                &test.tick_lower,
                test.tick_lower_index,
                &test.tick_upper,
                test.tick_upper_index,
                test.fee_growth_global_a,
                test.fee_growth_global_b,
            );
            assert_eq!(
                fee_growth_inside_a, test.expected_fee_growths_inside.0,
                "{} - fee_growth_inside_a",
                test.name
            );
            assert_eq!(
                fee_growth_inside_b, test.expected_fee_growths_inside.1,
                "{} - fee_growth_inside_b",
                test.name
            );
        }
    }

    #[test]
    fn test_next_reward_growths_inside() {
        struct Test<'a> {
            name: &'a str,
            tick_current_index: i32,
            tick_lower: Tick,
            tick_lower_index: i32,
            tick_upper: Tick,
            tick_upper_index: i32,
            reward_infos: [AiDexRewardInfo; NUM_REWARDS],
            expected_reward_growths_inside: [u128; NUM_REWARDS],
        }

        for test in [
            Test {
                name: "current tick index below ticks zero rewards",
                tick_lower: Tick {
                    initialized: true,
                    reward_growths_outside: [100, 666, 69420],
                    ..Default::default()
                },
                tick_lower_index: -100,
                tick_upper: Tick {
                    initialized: true,
                    reward_growths_outside: [100, 666, 69420],
                    ..Default::default()
                },
                tick_upper_index: 100,
                tick_current_index: -200,
                reward_infos: [
                    create_test_ai_dex_reward_info(1, 500, true),
                    create_test_ai_dex_reward_info(1, 1000, true),
                    create_test_ai_dex_reward_info(1, 70000, true),
                ],
                expected_reward_growths_inside: [0, 0, 0],
            },
            Test {
                name: "current tick index between ticks",
                tick_lower: Tick {
                    initialized: true,
                    reward_growths_outside: [200, 134, 480],
                    ..Default::default()
                },
                tick_lower_index: -100,
                tick_upper: Tick {
                    initialized: true,
                    reward_growths_outside: [100, 666, 69420],
                    ..Default::default()
                },
                tick_upper_index: 100,
                tick_current_index: 10,
                reward_infos: [
                    create_test_ai_dex_reward_info(1, 1000, true),
                    create_test_ai_dex_reward_info(1, 2000, true),
                    create_test_ai_dex_reward_info(1, 80000, true),
                ],
                expected_reward_growths_inside: [700, 1200, 10100],
            },
            Test {
                name: "current tick index above ticks",
                tick_lower: Tick {
                    reward_growths_outside: [200, 134, 480],
                    initialized: true,
                    ..Default::default()
                },
                tick_lower_index: -100,
                tick_upper: Tick {
                    initialized: true,
                    reward_growths_outside: [900, 1334, 10580],
                    ..Default::default()
                },
                tick_upper_index: 100,
                tick_current_index: 250,
                reward_infos: [
                    create_test_ai_dex_reward_info(1, 1000, true),
                    create_test_ai_dex_reward_info(1, 2000, true),
                    create_test_ai_dex_reward_info(1, 80000, true),
                ],
                expected_reward_growths_inside: [700, 1200, 10100],
            },
            Test {
                name: "uninitialized rewards no-op",
                tick_lower: Tick {
                    initialized: true,
                    reward_growths_outside: [200, 134, 480],
                    ..Default::default()
                },
                tick_lower_index: -100,
                tick_upper: Tick {
                    initialized: true,
                    reward_growths_outside: [900, 1334, 10580],
                    ..Default::default()
                },
                tick_upper_index: 100,
                tick_current_index: 250,
                reward_infos: [
                    create_test_ai_dex_reward_info(1, 1000, true),
                    create_test_ai_dex_reward_info(1, 2000, false),
                    create_test_ai_dex_reward_info(1, 80000, false),
                ],
                expected_reward_growths_inside: [700, 0, 0],
            },
        ] {
            // System under test
            let results = next_reward_growths_inside(
                test.tick_current_index,
                &test.tick_lower,
                test.tick_lower_index,
                &test.tick_upper,
                test.tick_upper_index,
                &test.reward_infos,
            );

            for i in 0..NUM_REWARDS {
                assert_eq!(
                    results[i], test.expected_reward_growths_inside[i],
                    "[{}] {} - reward growth value not equal",
                    i, test.name
                );
                assert_eq!(
                    results[i], test.expected_reward_growths_inside[i],
                    "[{}] {} - reward growth initialized flag not equal",
                    i, test.name
                );
            }
        }
    }

    #[test]
    fn test_next_tick_modify_liquidity_update() {
        #[derive(Default)]
        struct Test<'a> {
            name: &'a str,
            tick: Tick,
            tick_index: i32,
            tick_current_index: i32,
            fee_growth_global_a: u128,
            fee_growth_global_b: u128,
            reward_infos: [AiDexRewardInfo; NUM_REWARDS],
            liquidity_delta: i128,
            is_upper_tick: bool,
            expected_update: TickUpdate,
        }

        // AiDex rewards re-used in the tests
        let reward_infos = [
            AiDexRewardInfo {
                mint: Pubkey::new_unique(),
                emissions_per_second_x64: 1 << Q64_RESOLUTION,
                growth_global_x64: 100 << Q64_RESOLUTION,
                ..Default::default()
            },
            AiDexRewardInfo {
                mint: Pubkey::new_unique(),
                emissions_per_second_x64: 1 << Q64_RESOLUTION,
                growth_global_x64: 100 << Q64_RESOLUTION,
                ..Default::default()
            },
            AiDexRewardInfo {
                mint: Pubkey::new_unique(),
                emissions_per_second_x64: 1 << Q64_RESOLUTION,
                growth_global_x64: 100 << Q64_RESOLUTION,
                ..Default::default()
            },
        ];

        for test in [
            Test {
                name: "initialize lower tick with +liquidity, current < tick.index, growths not set",
                tick: Tick::default(),
                tick_index: 200,
                tick_current_index: 100,
                liquidity_delta: 42069,
                is_upper_tick: false,
                fee_growth_global_a: 100,
                fee_growth_global_b: 100,
                reward_infos,
                expected_update: TickUpdate {
                    initialized: true,
                    liquidity_net: 42069,
                    liquidity_gross: 42069,
                    ..Default::default()
                },
            },
            Test {
                name: "initialize lower tick with +liquidity, current >= tick.index, growths get set",
                tick: Tick::default(),
                tick_index: 200,
                tick_current_index: 300,
                liquidity_delta: 42069,
                is_upper_tick: false,
                fee_growth_global_a: 100,
                fee_growth_global_b: 100,
                reward_infos,
                expected_update: TickUpdate {
                    initialized: true,
                    liquidity_net: 42069,
                    liquidity_gross: 42069,
                    fee_growth_outside_a: 100,
                    fee_growth_outside_b: 100,
                    reward_growths_outside: [
                        100 << Q64_RESOLUTION,
                        100 << Q64_RESOLUTION,
                        100 << Q64_RESOLUTION,
                    ],
                },
                ..Default::default()
            },
            Test {
                name: "lower tick +liquidity already initialized, growths not set",
                tick: TickBuilder::default()
                    .initialized(true)
                    .liquidity_net(100)
                    .liquidity_gross(100)
                    .build(),
                tick_index: 200,
                tick_current_index: 100,
                liquidity_delta: 42069,
                is_upper_tick: false,
                fee_growth_global_a: 100,
                fee_growth_global_b: 100,
                reward_infos,
                expected_update: TickUpdate {
                    initialized: true,
                    liquidity_net: 42169,
                    liquidity_gross: 42169,
                    ..Default::default()
                },
                ..Default::default()
            },
            Test {
                name: "upper tick +liquidity already initialized, growths not set, liquidity net should be subtracted",
                tick: TickBuilder::default()
                    .initialized(true)
                    .liquidity_net(100000)
                    .liquidity_gross(100000)
                    .build(),
                tick_index: 200,
                tick_current_index: 100,
                liquidity_delta: 42069,
                is_upper_tick: true,
                expected_update: TickUpdate {
                    initialized: true,
                    liquidity_net:57931,
                    liquidity_gross: 142069,
                    ..Default::default()
                },
                ..Default::default()
            },
            Test {
                name: "upper tick -liquidity, growths not set, uninitialize tick",
                tick: TickBuilder::default()
                    .initialized(true)
                    .liquidity_net(-100000)
                    .liquidity_gross(100000)
                    .build(),
                tick_index: 200,
                tick_current_index: 100,
                liquidity_delta: -100000,
                is_upper_tick: true,
                expected_update: TickUpdate {
                    initialized: false,
                    liquidity_net: 0,
                    liquidity_gross: 0,
                    ..Default::default()
                },
                ..Default::default()
            },
            Test {
                name: "lower tick -liquidity, growths not set, initialized no change",
                tick: TickBuilder::default()
                    .initialized(true)
                    .liquidity_net(100000)
                    .liquidity_gross(200000)
                    .build(),
                tick_index: 200,
                tick_current_index: 100,
                liquidity_delta: -100000,
                is_upper_tick: false,
                expected_update: TickUpdate {
                    initialized: true,
                    liquidity_net: 0,
                    liquidity_gross: 100000,
                    ..Default::default()
                },
                ..Default::default()
            },
            Test {
                name: "liquidity delta zero is no-op",
                tick: TickBuilder::default()
                    .initialized(true)
                    .liquidity_net(100000)
                    .liquidity_gross(200000)
                    .build(),
                tick_index: 200,
                tick_current_index: 100,
                liquidity_delta: 0,
                is_upper_tick: false,
                expected_update: TickUpdate {
                    initialized: true,
                    liquidity_net: 100000,
                    liquidity_gross: 200000,
                    ..Default::default()
                },
                ..Default::default()
            },
            Test {
                name: "uninitialized rewards get set to zero values",
                tick: TickBuilder::default()
                    .initialized(true)
                    .reward_growths_outside([100, 200, 50])
                    .build(),
                tick_index: 200,
                tick_current_index: 1000,
                liquidity_delta: 42069,
                is_upper_tick: false,
                fee_growth_global_a: 100,
                fee_growth_global_b: 100,
                reward_infos: [
                    AiDexRewardInfo{
                        ..Default::default()
                    },
                    AiDexRewardInfo{
                        mint: Pubkey::new_unique(),
                        emissions_per_second_x64: 1,
                        growth_global_x64: 250,
                        ..Default::default()
                    },
                    AiDexRewardInfo{
                        ..Default::default()
                    }
                ],
                expected_update: TickUpdate {
                    initialized: true,
                    fee_growth_outside_a: 100,
                    fee_growth_outside_b: 100,
                    liquidity_net: 42069,
                    liquidity_gross: 42069,
                    reward_growths_outside: [0, 250, 0],
                    ..Default::default()
                },
            }
        ] {
            // System under test
            let update = next_tick_modify_liquidity_update(
                &test.tick,
                test.tick_index,
                test.tick_current_index,
                test.fee_growth_global_a,
                test.fee_growth_global_b,
                &test.reward_infos,
                test.liquidity_delta,
                test.is_upper_tick,
            )
            .unwrap();

            assert_eq!(
                update.initialized, test.expected_update.initialized,
                "{}: initialized invalid",
                test.name
            );
            assert_eq!(
                update.liquidity_net, test.expected_update.liquidity_net,
                "{}: liquidity_net invalid",
                test.name
            );
            assert_eq!(
                update.liquidity_gross, test.expected_update.liquidity_gross,
                "{}: liquidity_gross invalid",
                test.name
            );
            assert_eq!(
                update.fee_growth_outside_a, test.expected_update.fee_growth_outside_a,
                "{}: fee_growth_outside_a invalid",
                test.name
            );
            assert_eq!(
                update.fee_growth_outside_b, test.expected_update.fee_growth_outside_b,
                "{}: fee_growth_outside_b invalid",
                test.name
            );
            assert_eq!(
                update.reward_growths_outside, test.expected_update.reward_growths_outside,
                "{}: reward_growths_outside invalid",
                test.name
            );
        }
    }

    #[test]
    fn test_next_tick_modify_liquidity_update_errors() {
        struct Test<'a> {
            name: &'a str,
            tick: Tick,
            tick_index: i32,
            tick_current_index: i32,
            liquidity_delta: i128,
            is_upper_tick: bool,
            expected_error: ErrorCode,
        }

        for test in [
            Test {
                name: "liquidity gross overflow",
                tick: TickBuilder::default().liquidity_gross(u128::MAX).build(),
                tick_index: 0,
                tick_current_index: 10,
                liquidity_delta: i128::MAX,
                is_upper_tick: false,
                expected_error: ErrorCode::LiquidityOverflowError,
            },
            Test {
                name: "liquidity gross underflow",
                tick: Tick::default(),
                tick_index: 0,
                tick_current_index: 10,
                liquidity_delta: -100,
                is_upper_tick: false,
                expected_error: ErrorCode::LiquidityUnderflowError,
            },
            Test {
                name: "liquidity net overflow from subtracting negative delta",
                tick: TickBuilder::default()
                    .liquidity_gross(i128::MAX as u128)
                    .liquidity_net(i128::MAX)
                    .build(),
                tick_index: 0,
                tick_current_index: 10,
                liquidity_delta: -(i128::MAX - 1),
                is_upper_tick: true,
                expected_error: ErrorCode::TickLiquidityNetError,
            },
            Test {
                name: "liquidity net underflow",
                tick: TickBuilder::default()
                    .liquidity_gross(10000)
                    .liquidity_net(i128::MAX)
                    .build(),
                tick_index: 0,
                tick_current_index: 10,
                liquidity_delta: i128::MAX,
                is_upper_tick: false,
                expected_error: ErrorCode::TickLiquidityNetError,
            },
        ] {
            // System under test
            let err = next_tick_modify_liquidity_update(
                &test.tick,
                test.tick_index,
                test.tick_current_index,
                0,
                0,
                &[AiDexRewardInfo::default(); NUM_REWARDS],
                test.liquidity_delta,
                test.is_upper_tick,
            )
            .unwrap_err();

            assert_eq!(err, test.expected_error, "{}", test.name);
        }
    }

    #[test]
    fn test_next_tick_cross_update() {
        struct Test<'a> {
            name: &'a str,
            tick: Tick,
            fee_growth_global_a: u128,
            fee_growth_global_b: u128,
            reward_infos: [AiDexRewardInfo; NUM_REWARDS],
            expected_update: TickUpdate,
        }

        for test in [Test {
            name: "growths set properly (inverted)",
            tick: TickBuilder::default()
                .fee_growth_outside_a(1000)
                .fee_growth_outside_b(1000)
                .reward_growths_outside([500, 250, 100])
                .build(),
            fee_growth_global_a: 2500,
            fee_growth_global_b: 6750,
            reward_infos: [
                AiDexRewardInfo {
                    mint: Pubkey::new_unique(),
                    emissions_per_second_x64: 1,
                    growth_global_x64: 1000,
                    ..Default::default()
                },
                AiDexRewardInfo {
                    mint: Pubkey::new_unique(),
                    emissions_per_second_x64: 1,
                    growth_global_x64: 1000,
                    ..Default::default()
                },
                AiDexRewardInfo {
                    mint: Pubkey::new_unique(),
                    emissions_per_second_x64: 1,
                    growth_global_x64: 1000,
                    ..Default::default()
                },
            ],
            expected_update: TickUpdate {
                fee_growth_outside_a: 1500,
                fee_growth_outside_b: 5750,
                reward_growths_outside: [500, 750, 900],
                ..Default::default()
            },
        }] {
            // System under test
            let update = next_tick_cross_update(
                &test.tick,
                test.fee_growth_global_a,
                test.fee_growth_global_b,
                &test.reward_infos,
            )
            .unwrap();

            assert_eq!(
                update.fee_growth_outside_a, test.expected_update.fee_growth_outside_a,
                "{}: fee_growth_outside_a invalid",
                test.name
            );
            assert_eq!(
                update.fee_growth_outside_b, test.expected_update.fee_growth_outside_b,
                "{}: fee_growth_outside_b invalid",
                test.name
            );

            let reward_growths_outside = update.reward_growths_outside;
            let expected_growths_outside = test.expected_update.reward_growths_outside;
            for i in 0..NUM_REWARDS {
                assert_eq!(
                    reward_growths_outside[i], expected_growths_outside[i],
                    "{}: reward_growth[{}] invalid",
                    test.name, i
                );
            }
        }
    }
}
