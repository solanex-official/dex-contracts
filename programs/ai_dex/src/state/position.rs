use anchor_lang::prelude::*;

use crate::{errors::ErrorCode, math::FULL_RANGE_ONLY_TICK_SPACING_THRESHOLD, state::NUM_REWARDS};

use super::{Tick, AiDexPool};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Copy)]
pub struct OpenPositionBumps {
    pub position_bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default, Copy)]
pub struct OpenPositionWithMetadataBumps {
    pub position_bump: u8,
    pub metadata_bump: u8,
}

#[account]
#[derive(Default)]
pub struct Position {
    pub ai_dex_pool: Pubkey,     // 32
    pub position_mint: Pubkey, // 32
    pub liquidity: u128,       // 16
    pub tick_lower_index: i32, // 4
    pub tick_upper_index: i32, // 4

    // Q64.64
    pub fee_growth_checkpoint_a: u128, // 16
    pub fee_owed_a: u64,               // 8
    // Q64.64
    pub fee_growth_checkpoint_b: u128, // 16
    pub fee_owed_b: u64,               // 8

    pub reward_infos: [PositionRewardInfo; NUM_REWARDS], // 72

    pub is_reinvestment_on: bool, // 1
}

/// Represents a position in the AiDex program.
impl Position {
    /// The length of a position in bytes.
    pub const LEN: usize = 8 + 136 + 72 + 1;

    /// Checks if a position is empty.
    ///
    /// A position is considered empty if its liquidity is zero and all fees and rewards owed are zero.
    ///
    /// # Arguments
    ///
    /// * `position` - A reference to the position to check.
    ///
    /// # Returns
    ///
    /// * `true` if the position is empty, `false` otherwise.
    pub fn is_position_empty(position: &Position) -> bool {
        position.liquidity == 0 && 
        position.fee_owed_a == 0 && 
        position.fee_owed_b == 0 && 
        position.reward_infos.iter().all(
            |reward| reward.amount_owed == 0
        )
    }

    /// Updates the position with the given position update.
    ///
    /// # Arguments
    ///
    /// * `update` - A reference to the position update.
    pub fn update(&mut self, update: &PositionUpdate) {
        self.liquidity = update.liquidity;
        self.fee_growth_checkpoint_a = update.fee_growth_checkpoint_a;
        self.fee_growth_checkpoint_b = update.fee_growth_checkpoint_b;
        self.fee_owed_a = update.fee_owed_a;
        self.fee_owed_b = update.fee_owed_b;
        self.reward_infos = update.reward_infos;
    }

    /// Opens a position in the AiDex program.
    ///
    /// # Arguments
    ///
    /// * `ai_dex` - A reference to the AiDex account.
    /// * `position_mint` - The mint of the position.
    /// * `tick_lower_index` - The lower tick index of the position.
    /// * `tick_upper_index` - The upper tick index of the position.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the position was opened successfully.
    /// * An error if the tick indexes are invalid or the pool is full range only.
    pub fn open_position(
        &mut self,
        ai_dex: &AccountLoader<AiDexPool>,
        position_mint: Pubkey,
        tick_lower_index: i32,
        tick_upper_index: i32,
        is_reinvestment_on: bool,
    ) -> Result<()> {
        let ai_dex_data = ai_dex.load()?;

        if !Tick::check_is_usable_tick(tick_lower_index, ai_dex_data.tick_spacing)
            || !Tick::check_is_usable_tick(tick_upper_index, ai_dex_data.tick_spacing)
            || tick_lower_index >= tick_upper_index
        {
            return Err(ErrorCode::InvalidTickIndexError.into());
        }

        // On tick spacing >= 2^15, should only be able to open full range positions
        if ai_dex_data.tick_spacing >= FULL_RANGE_ONLY_TICK_SPACING_THRESHOLD {
            let (full_range_lower_index, full_range_upper_index) = Tick::full_range_indexes(ai_dex_data.tick_spacing);
            if tick_lower_index != full_range_lower_index
                || tick_upper_index != full_range_upper_index
            {
                return Err(ErrorCode::FullRangeOnlyPoolError.into());
            }
        }

        self.ai_dex_pool = ai_dex.key();
        self.position_mint = position_mint;

        self.tick_lower_index = tick_lower_index;
        self.tick_upper_index = tick_upper_index;

        self.is_reinvestment_on = is_reinvestment_on;
        Ok(())
    }

    /// Resets the fees owed by the position to zero.
    pub fn reset_fees_owed(&mut self) {
        self.fee_owed_a = 0;
        self.fee_owed_b = 0;
    }

    pub fn subtract_fees_owed(&mut self, fee_owed_a: u64, fee_owed_b: u64) {
        self.fee_owed_a = self.fee_owed_a.saturating_sub(fee_owed_a);
        self.fee_owed_b = self.fee_owed_b.saturating_sub(fee_owed_b);
    }

    /// Updates the amount owed for a specific reward in the position.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the reward in the position.
    /// * `amount_owed` - The new amount owed for the reward.
    pub fn update_reward_owed(&mut self, index: usize, amount_owed: u64) {
        self.reward_infos[index].amount_owed = amount_owed;
    }
}

#[derive(Copy, Clone, AnchorSerialize, AnchorDeserialize, Default, Debug, PartialEq)]
pub struct PositionRewardInfo {
    // Q64.64
    pub growth_inside_checkpoint: u128,
    pub amount_owed: u64,
}

#[derive(Default, Debug, PartialEq, AnchorSerialize, AnchorDeserialize, Copy, Clone)]
pub struct PositionUpdate {
    pub liquidity: u128,
    pub fee_growth_checkpoint_a: u128,
    pub fee_owed_a: u64,
    pub fee_growth_checkpoint_b: u128,
    pub fee_owed_b: u64,
    pub reward_infos: [PositionRewardInfo; NUM_REWARDS],
}

#[cfg(test)]
mod is_position_empty_tests {
    use super::*;
    use crate::constants::test_constants::*;

    pub fn build_test_position(
        liquidity: u128,
        fee_owed_a: u64,
        fee_owed_b: u64,
        reward_owed_0: u64,
        reward_owed_1: u64,
        reward_owed_2: u64,
    ) -> Position {
        Position {
            ai_dex_pool: test_program_id(),
            position_mint: test_program_id(),
            liquidity,
            tick_lower_index: 0,
            tick_upper_index: 0,
            fee_growth_checkpoint_a: 0,
            fee_owed_a,
            fee_growth_checkpoint_b: 0,
            fee_owed_b,
            reward_infos: [
                PositionRewardInfo {
                    growth_inside_checkpoint: 0,
                    amount_owed: reward_owed_0,
                },
                PositionRewardInfo {
                    growth_inside_checkpoint: 0,
                    amount_owed: reward_owed_1,
                },
                PositionRewardInfo {
                    growth_inside_checkpoint: 0,
                    amount_owed: reward_owed_2,
                },
            ],
            is_reinvestment_on: false,
        }
    }

    #[test]
    fn test_position_empty() {
        let pos = build_test_position(0, 0, 0, 0, 0, 0);
        assert_eq!(Position::is_position_empty(&pos), true);
    }

    #[test]
    fn test_liquidity_non_zero() {
        let pos = build_test_position(100, 0, 0, 0, 0, 0);
        assert_eq!(Position::is_position_empty(&pos), false);
    }

    #[test]
    fn test_fee_a_non_zero() {
        let pos = build_test_position(0, 100, 0, 0, 0, 0);
        assert_eq!(Position::is_position_empty(&pos), false);
    }

    #[test]
    fn test_fee_b_non_zero() {
        let pos = build_test_position(0, 0, 100, 0, 0, 0);
        assert_eq!(Position::is_position_empty(&pos), false);
    }

    #[test]
    fn test_reward_0_non_zero() {
        let pos = build_test_position(0, 0, 0, 100, 0, 0);
        assert_eq!(Position::is_position_empty(&pos), false);
    }

    #[test]
    fn test_reward_1_non_zero() {
        let pos = build_test_position(0, 0, 0, 0, 100, 0);
        assert_eq!(Position::is_position_empty(&pos), false);
    }

    #[test]
    fn test_reward_2_non_zero() {
        let pos = build_test_position(0, 0, 0, 0, 0, 100);
        assert_eq!(Position::is_position_empty(&pos), false);
    }
}

#[cfg(test)]
pub mod position_builder {
    use anchor_lang::prelude::Pubkey;

    use super::{Position, PositionRewardInfo};
    use crate::state::NUM_REWARDS;

    #[derive(Default)]
    pub struct PositionBuilder {
        liquidity: u128,

        tick_lower_index: i32,
        tick_upper_index: i32,

        // Q64.64
        fee_growth_checkpoint_a: u128,
        fee_owed_a: u64,
        // Q64.64
        fee_growth_checkpoint_b: u128,
        fee_owed_b: u64,

        // Size should equal state::NUM_REWARDS
        reward_infos: [PositionRewardInfo; NUM_REWARDS],
    }

    impl PositionBuilder {
        pub fn new(tick_lower_index: i32, tick_upper_index: i32) -> Self {
            Self {
                tick_lower_index,
                tick_upper_index,
                reward_infos: [PositionRewardInfo::default(); NUM_REWARDS],
                ..Default::default()
            }
        }

        pub fn liquidity(mut self, liquidity: u128) -> Self {
            self.liquidity = liquidity;
            self
        }

        pub fn fee_growth_checkpoint_a(mut self, fee_growth_checkpoint_a: u128) -> Self {
            self.fee_growth_checkpoint_a = fee_growth_checkpoint_a;
            self
        }

        pub fn fee_growth_checkpoint_b(mut self, fee_growth_checkpoint_b: u128) -> Self {
            self.fee_growth_checkpoint_b = fee_growth_checkpoint_b;
            self
        }

        pub fn fee_owed_a(mut self, fee_owed_a: u64) -> Self {
            self.fee_owed_a = fee_owed_a;
            self
        }

        pub fn fee_owed_b(mut self, fee_owed_b: u64) -> Self {
            self.fee_owed_b = fee_owed_b;
            self
        }

        pub fn reward_info(mut self, index: usize, reward_info: PositionRewardInfo) -> Self {
            self.reward_infos[index] = reward_info;
            self
        }

        pub fn reward_infos(mut self, reward_infos: [PositionRewardInfo; NUM_REWARDS]) -> Self {
            self.reward_infos = reward_infos;
            self
        }

        pub fn build(self) -> Position {
            Position {
                ai_dex_pool: Pubkey::new_unique(),
                position_mint: Pubkey::new_unique(),
                liquidity: self.liquidity,
                fee_growth_checkpoint_a: self.fee_growth_checkpoint_a,
                fee_growth_checkpoint_b: self.fee_growth_checkpoint_b,
                fee_owed_a: self.fee_owed_a,
                fee_owed_b: self.fee_owed_b,
                reward_infos: self.reward_infos,
                tick_lower_index: self.tick_lower_index,
                tick_upper_index: self.tick_upper_index,
                ..Default::default()
            }
        }
    }
}
