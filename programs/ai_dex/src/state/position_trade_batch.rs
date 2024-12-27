use crate::errors::ErrorCode;
use anchor_lang::prelude::*;

pub const POSITION_BITMAP_USIZE: usize = 32;
pub const POSITION_TRADE_BATCH_SIZE: u16 = 8 * POSITION_BITMAP_USIZE as u16;

#[account]
#[derive(Default)]
pub struct PositionTradeBatch {
    pub position_trade_batch_mint: Pubkey, // 32
    pub position_bitmap: [u8; POSITION_BITMAP_USIZE], // 32
                                      // 64 RESERVE
}

/// Represents a position trade batch.
impl PositionTradeBatch {
    /// The length of the position trade batch in bytes.
    pub const LEN: usize = 8 + 32 + 32 + 64;

    /// Initializes the position trade batch with the given mint.
    ///
    /// # Arguments
    ///
    /// * `position_trade_batch_mint` - The mint of the position trade batch.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn initialize(&mut self, position_trade_batch_mint: Pubkey) -> Result<()> {
        self.position_trade_batch_mint = position_trade_batch_mint;
        // position_bitmap is initialized using Default trait
        Ok(())
    }

    /// Checks if the position trade batch is deletable.
    ///
    /// Returns `true` if the position bitmap is empty, indicating that all trade batch positions are closed.
    pub fn is_deletable(&self) -> bool {
        self.position_bitmap.iter().all(|&bitmap| bitmap == 0)
    }

    /// Opens a trade batch position in the position trade batch.
    ///
    /// # Arguments
    ///
    /// * `trade_batch_index` - The index of the trade batch position to open.
    ///
    /// # Errors
    ///
    /// Returns an error if the trade batch index is invalid or if the position is already opened.
    pub fn open_trade_batch_position(&mut self, trade_batch_index: u16) -> Result<()> {
        self.update_bitmap(trade_batch_index, true)
    }

    /// Closes a trade batch position in the position trade batch.
    ///
    /// # Arguments
    ///
    /// * `trade_batch_index` - The index of the trade batch position to close.
    ///
    /// # Errors
    ///
    /// Returns an error if the trade batch index is invalid or if the position is already closed.
    pub fn close_trade_batch_position(&mut self, trade_batch_index: u16) -> Result<()> {
        self.update_bitmap(trade_batch_index, false)
    }

    /// Updates the position bitmap based on the trade batch index and open flag.
    ///
    /// # Arguments
    ///
    /// * `trade_batch_index` - The index of the trade batch position to update.
    /// * `open` - A flag indicating whether to open or close the position.
    ///
    /// # Errors
    ///
    /// Returns an error if the trade batch index is invalid or if the position is already opened/closed.
    fn update_bitmap(&mut self, trade_batch_index: u16, open: bool) -> Result<()> {
        if !PositionTradeBatch::is_valid_trade_batch_index(trade_batch_index) {
            return Err(ErrorCode::InvalidTradeBatchIndexError.into());
        }

        let bitmap_index = trade_batch_index / 8;
        let bitmap_offset = trade_batch_index % 8;
        let bitmap = &mut self.position_bitmap[bitmap_index as usize];

        let mask = 1 << bitmap_offset;
        let bit = *bitmap & mask;

        if open && bit != 0 {
            return Err(ErrorCode::PositionAlreadyOpenedError.into());
        }
        if !open && bit == 0 {
            return Err(ErrorCode::PositionAlreadyClosedError.into());
        }

        *bitmap ^= mask;

        Ok(())
    }

    /// Checks if the trade batch index is valid.
    ///
    /// # Arguments
    ///
    /// * `trade_batch_index` - The index of the trade batch position.
    ///
    /// # Returns
    ///
    /// Returns `true` if the trade batch index is less than `POSITION_TRADE_BATCH_SIZE`.
    fn is_valid_trade_batch_index(trade_batch_index: u16) -> bool {
        trade_batch_index < POSITION_TRADE_BATCH_SIZE
    }
}

#[cfg(test)]
mod position_trade_batch_initialize_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_default() {
        let position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };
        assert_eq!(position_trade_batch.position_trade_batch_mint, Pubkey::default());
        for bitmap in position_trade_batch.position_bitmap.iter() {
            assert_eq!(*bitmap, 0);
        }
    }

    #[test]
    fn test_initialize() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };
        let position_trade_batch_mint =
            Pubkey::from_str("8y6jyKgGcfDHzi3DgQn3ZHVimjawCU5o7Pr46RrB81fV").unwrap();

        let result = position_trade_batch.initialize(position_trade_batch_mint);
        assert!(result.is_ok());

        assert_eq!(position_trade_batch.position_trade_batch_mint, position_trade_batch_mint);
        for bitmap in position_trade_batch.position_bitmap.iter() {
            assert_eq!(*bitmap, 0);
        }
    }
}

#[cfg(test)]
mod position_trade_batch_is_deletable_tests {
    use super::*;

    #[test]
    fn test_default_is_deletable() {
        let position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };
        assert!(position_trade_batch.is_deletable());
    }

    #[test]
    fn test_each_bit_detectable() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };
        for trade_batch_index in 0..POSITION_TRADE_BATCH_SIZE {
            let index = trade_batch_index / 8;
            let offset = trade_batch_index % 8;
            position_trade_batch.position_bitmap[index as usize] = 1 << offset;
            assert!(!position_trade_batch.is_deletable());
            position_trade_batch.position_bitmap[index as usize] = 0;
            assert!(position_trade_batch.is_deletable());
        }
    }
}

#[cfg(test)]
mod position_trade_batch_open_and_close_tests {
    use super::*;

    #[test]
    fn test_open_and_close_zero() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };

        let r1 = position_trade_batch.open_trade_batch_position(0);
        assert!(r1.is_ok());
        assert_eq!(position_trade_batch.position_bitmap[0], 1);

        let r2 = position_trade_batch.close_trade_batch_position(0);
        assert!(r2.is_ok());
        assert_eq!(position_trade_batch.position_bitmap[0], 0);
    }

    #[test]
    fn test_open_and_close_middle() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };

        let r1 = position_trade_batch.open_trade_batch_position(130);
        assert!(r1.is_ok());
        assert_eq!(position_trade_batch.position_bitmap[16], 4);

        let r2 = position_trade_batch.close_trade_batch_position(130);
        assert!(r2.is_ok());
        assert_eq!(position_trade_batch.position_bitmap[16], 0);
    }

    #[test]
    fn test_open_and_close_max() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };

        let r1 = position_trade_batch.open_trade_batch_position(POSITION_TRADE_BATCH_SIZE - 1);
        assert!(r1.is_ok());
        assert_eq!(
            position_trade_batch.position_bitmap[POSITION_BITMAP_USIZE - 1],
            128
        );

        let r2 = position_trade_batch.close_trade_batch_position(POSITION_TRADE_BATCH_SIZE - 1);
        assert!(r2.is_ok());
        assert_eq!(
            position_trade_batch.position_bitmap[POSITION_BITMAP_USIZE - 1],
            0
        );
    }

    #[test]
    fn test_double_open_should_be_failed() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };

        let r1 = position_trade_batch.open_trade_batch_position(0);
        assert!(r1.is_ok());

        let r2 = position_trade_batch.open_trade_batch_position(0);
        assert!(r2.is_err());
    }

    #[test]
    fn test_double_close_should_be_failed() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };

        let r1 = position_trade_batch.open_trade_batch_position(0);
        assert!(r1.is_ok());

        let r2 = position_trade_batch.close_trade_batch_position(0);
        assert!(r2.is_ok());

        let r3 = position_trade_batch.close_trade_batch_position(0);
        assert!(r3.is_err());
    }

    #[test]
    fn test_all_open_and_all_close() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };

        for trade_batch_index in 0..POSITION_TRADE_BATCH_SIZE {
            let r = position_trade_batch.open_trade_batch_position(trade_batch_index);
            assert!(r.is_ok());
        }

        for bitmap in position_trade_batch.position_bitmap.iter() {
            assert_eq!(*bitmap, 255);
        }

        for trade_batch_index in 0..POSITION_TRADE_BATCH_SIZE {
            let r = position_trade_batch.close_trade_batch_position(trade_batch_index);
            assert!(r.is_ok());
        }

        for bitmap in position_trade_batch.position_bitmap.iter() {
            assert_eq!(*bitmap, 0);
        }
    }

    #[test]
    fn test_open_trade_batch_index_out_of_bounds() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };

        for trade_batch_index in POSITION_TRADE_BATCH_SIZE..u16::MAX {
            let r = position_trade_batch.open_trade_batch_position(trade_batch_index);
            assert!(r.is_err());
        }
    }

    #[test]
    fn test_close_trade_batch_index_out_of_bounds() {
        let mut position_trade_batch = PositionTradeBatch {
            ..Default::default()
        };

        for trade_batch_index in POSITION_TRADE_BATCH_SIZE..u16::MAX {
            let r = position_trade_batch.close_trade_batch_position(trade_batch_index);
            assert!(r.is_err());
        }
    }
}
