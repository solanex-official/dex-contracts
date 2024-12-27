use crate::errors::ErrorCode;
use crate::math::{add_liquidity_delta, checked_mul_div};
use crate::state::*;

// Calculates the next global reward growth variables based on the given timestamp.
// The provided timestamp must be greater than or equal to the last updated timestamp.
pub fn next_ai_dex_reward_infos(
    ai_dex: &AiDexPool,
    next_timestamp: u64,
) -> Result<[AiDexRewardInfo; NUM_REWARDS], ErrorCode> {
    let curr_timestamp = ai_dex.reward_last_updated_timestamp;

    // Check if the next timestamp is earlier than the current timestamp
    if next_timestamp < curr_timestamp {
        return Err(ErrorCode::InvalidTimestampError.into());
    }

    // No-op if there is no liquidity or no change in timestamp
    if ai_dex.liquidity == 0 || next_timestamp == curr_timestamp {
        return Ok(ai_dex.reward_infos);
    }

    // Calculate new global reward growth
    let mut next_reward_infos = ai_dex.reward_infos;
    let time_delta = u128::from(next_timestamp - curr_timestamp);

    // Iterate through each reward info and calculate the new reward growth
    for reward_info in &mut next_reward_infos {
        if !reward_info.initialized() {
            continue;
        }

        // Calculate the new reward growth delta.
        // If the calculation overflows, set the delta value to zero.
        // This will halt reward distributions for this reward.
        let reward_growth_delta = checked_mul_div(
            time_delta,
            reward_info.emissions_per_second_x64,
            ai_dex.liquidity,
        )
        .unwrap_or(0);

        // Add the reward growth delta to the global reward growth.
        reward_info.growth_global_x64 = reward_info.growth_global_x64.wrapping_add(reward_growth_delta);
    }

    Ok(next_reward_infos)
}

// Calculates the next global liquidity for an AiDex depending on its position relative
// to the lower and upper tick indexes and the liquidity_delta.
pub fn next_ai_dex_liquidity(
    ai_dex: &AiDexPool,
    tick_upper_index: i32,
    tick_lower_index: i32,
    liquidity_delta: i128,
) -> Result<u128, ErrorCode> {
    // Check if the AiDex is within the tick range
    if ai_dex.tick_current_index < tick_upper_index && ai_dex.tick_current_index >= tick_lower_index {
        // Add the liquidity delta to the current liquidity
        let new_liquidity = add_liquidity_delta(ai_dex.liquidity, liquidity_delta)?;
        Ok(new_liquidity)
    } else {
        // Return the current liquidity if the AiDex is not within the tick range
        Ok(ai_dex.liquidity)
    }
}

#[cfg(test)]
mod ai_dex_orchestrator_tests {

    use anchor_lang::prelude::Pubkey;

    use crate::orchestrator::ai_dex_orchestrator::next_ai_dex_reward_infos;
    use crate::math::Q64_RESOLUTION;
    use crate::state::ai_dex::AiDexRewardInfo;
    use crate::state::ai_dex::NUM_REWARDS;
    use crate::state::ai_dex_builder::AiDexBuilder;
    use crate::state::AiDexPool;

    // Initializes a ai_dex for testing with all the rewards initialized
    fn init_test_ai_dex(liquidity: u128, reward_last_updated_timestamp: u64) -> AiDexPool {
        AiDexBuilder::new()
            .liquidity(liquidity)
            .reward_last_updated_timestamp(reward_last_updated_timestamp) // Jan 1 2021 EST
            .reward_infos([
                AiDexRewardInfo {
                    mint: Pubkey::new_unique(),
                    emissions_per_second_x64: 10 << Q64_RESOLUTION,
                    growth_global_x64: 100 << Q64_RESOLUTION,
                    ..Default::default()
                },
                AiDexRewardInfo {
                    mint: Pubkey::new_unique(),
                    emissions_per_second_x64: 0b11 << (Q64_RESOLUTION - 1), // 1.5
                    growth_global_x64: 200 << Q64_RESOLUTION,
                    ..Default::default()
                },
                AiDexRewardInfo {
                    mint: Pubkey::new_unique(),
                    emissions_per_second_x64: 1 << (Q64_RESOLUTION - 1), // 0.5
                    growth_global_x64: 300 << Q64_RESOLUTION,
                    ..Default::default()
                },
            ])
            .build()
    }

    #[test]
    fn test_next_ai_dex_reward_infos_zero_liquidity_no_op() {
        let ai_dex = init_test_ai_dex(0, 1577854800);

        let result = next_ai_dex_reward_infos(&ai_dex, 1577855800);
        assert_eq!(
            AiDexRewardInfo::to_reward_growths(&result.unwrap()),
            [
                100 << Q64_RESOLUTION,
                200 << Q64_RESOLUTION,
                300 << Q64_RESOLUTION
            ]
        );
    }

    #[test]
    fn test_next_ai_dex_reward_infos_same_timestamp_no_op() {
        let ai_dex = init_test_ai_dex(100, 1577854800);

        let result = next_ai_dex_reward_infos(&ai_dex, 1577854800);
        assert_eq!(
            AiDexRewardInfo::to_reward_growths(&result.unwrap()),
            [
                100 << Q64_RESOLUTION,
                200 << Q64_RESOLUTION,
                300 << Q64_RESOLUTION
            ]
        );
    }

    #[test]
    #[should_panic(expected = "InvalidTimestampError")]
    fn test_next_ai_dex_reward_infos_invalid_timestamp() {
        let ai_dex = &AiDexBuilder::new()
            .liquidity(100)
            .reward_last_updated_timestamp(1577854800) // Jan 1 2020 EST
            .build();

        // New timestamp is earlier than the last updated timestamp
        next_ai_dex_reward_infos(ai_dex, 1577768400).unwrap(); // Dec 31 2019 EST
    }

    #[test]
    fn test_next_ai_dex_reward_infos_no_initialized_rewards() {
        let ai_dex = &AiDexBuilder::new()
            .liquidity(100)
            .reward_last_updated_timestamp(1577854800) // Jan 1 2021 EST
            .build();

        let new_timestamp = 1577854800 + 300;
        let result = next_ai_dex_reward_infos(ai_dex, new_timestamp).unwrap();
        assert_eq!(AiDexRewardInfo::to_reward_growths(&result), [0, 0, 0]);
    }

    #[test]
    fn test_next_ai_dex_reward_infos_some_initialized_rewards() {
        let ai_dex = &AiDexBuilder::new()
            .liquidity(100)
            .reward_last_updated_timestamp(1577854800) // Jan 1 2021 EST
            .reward_info(
                0,
                AiDexRewardInfo {
                    mint: Pubkey::new_unique(),
                    emissions_per_second_x64: 1 << Q64_RESOLUTION,
                    ..Default::default()
                },
            )
            .build();

        let new_timestamp = 1577854800 + 300;
        let result = next_ai_dex_reward_infos(ai_dex, new_timestamp).unwrap();
        let growth_global_x64_result_0 = result[0].growth_global_x64;
        assert_eq!(growth_global_x64_result_0, 3 << Q64_RESOLUTION);
        for i in 1..NUM_REWARDS {
            let growth_global_x64_ai_dex = ai_dex.reward_infos[i].growth_global_x64;
            assert_eq!(growth_global_x64_ai_dex, 0);
        }
    }

    #[test]
    fn test_next_ai_dex_reward_infos_delta_zero_on_overflow() {
        let ai_dex = &AiDexBuilder::new()
            .liquidity(100)
            .reward_last_updated_timestamp(0)
            .reward_info(
                0,
                AiDexRewardInfo {
                    mint: Pubkey::new_unique(),
                    emissions_per_second_x64: u128::MAX,
                    growth_global_x64: 100,
                    ..Default::default()
                },
            )
            .build();

        let new_timestamp = i64::MAX as u64;
        let result = next_ai_dex_reward_infos(ai_dex, new_timestamp).unwrap();
        let growth_global_x64_result_0 = result[0].growth_global_x64;
        assert_eq!(growth_global_x64_result_0, 100);
    }

    #[test]
    fn test_next_ai_dex_reward_infos_all_initialized_rewards() {
        let ai_dex = init_test_ai_dex(100, 1577854800);

        let new_timestamp = 1577854800 + 300;
        let result = next_ai_dex_reward_infos(&ai_dex, new_timestamp).unwrap();
        let growth_global_x64_result_0 = result[0].growth_global_x64;
        assert_eq!(growth_global_x64_result_0, 130 << Q64_RESOLUTION);
        let growth_global_x64_result_1 = result[1].growth_global_x64;
        assert_eq!(
            growth_global_x64_result_1,
            0b110011001 << (Q64_RESOLUTION - 1) // 204.5
        );
        let growth_global_x64_result_2 = result[2].growth_global_x64;
        assert_eq!(
            growth_global_x64_result_2,
            0b1001011011 << (Q64_RESOLUTION - 1) // 301.5
        );
    }
}
