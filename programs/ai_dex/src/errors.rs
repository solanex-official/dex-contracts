use std::num::TryFromIntError;

use anchor_lang::prelude::*;

#[error_code]
#[derive(PartialEq)]
pub enum ErrorCode {
    #[msg("Failed to convert enum value.")]
    EnumConversionError, // 0x1770 (6000)
    #[msg("Invalid start tick index.")]
    InvalidStartTickIndex, // 0x1771 (6001)
    #[msg("Tick array already exists in this pool.")]
    TickArrayAlreadyExists, // 0x1772 (6002)
    #[msg("Tick array index out of bounds.")]
    TickArrayIndexOutOfBounds, // 0x1773 (6003)
    #[msg("Unsupported tick spacing.")]
    UnsupportedTickSpacing, // 0x1774 (6004)
    #[msg("Position is not empty and cannot be closed.")]
    NonEmptyPositionCloseError, // 0x1775 (6005)
    #[msg("Division by zero is not allowed.")]
    DivisionByZeroError, // 0x1776 (6006)
    #[msg("Failed to cast number to BigInt.")]
    BigIntCastError, // 0x1777 (6007)
    #[msg("Failed to downcast number.")]
    NumberDowncastError, // 0x1778 (6008)
    #[msg("Tick not found in tick array.")]
    TickNotFoundError, // 0x1779 (6009)
    #[msg("Tick index is out of bounds or uninitializable.")]
    InvalidTickIndexError, // 0x177a (6010)
    #[msg("Sqrt price is out of bounds.")]
    SqrtPriceOutOfBoundsError, // 0x177b (6011)
    #[msg("Liquidity amount must be greater than zero.")]
    ZeroLiquidityError, // 0x177c (6012)
    #[msg("Liquidity amount exceeds maximum allowed.")]
    ExcessiveLiquidityError, // 0x177d (6013)
    #[msg("Liquidity overflow error.")]
    LiquidityOverflowError, // 0x177e (6014)
    #[msg("Liquidity underflow error.")]
    LiquidityUnderflowError, // 0x177f (6015)
    #[msg("Tick liquidity net overflow or underflow.")]
    TickLiquidityNetError, // 0x1780 (6016)
    #[msg("Exceeded maximum token limit.")]
    TokenLimitExceededError, // 0x1781 (6017)
    #[msg("Token amount below minimum required.")]
    TokenAmountBelowMinimumError, // 0x1782 (6018)
    #[msg("Invalid or missing delegate for position token account.")]
    InvalidDelegateError, // 0x1783 (6019)
    #[msg("Position token amount must be exactly 1.")]
    InvalidPositionTokenAmountError, // 0x1784 (6020)
    #[msg("Timestamp conversion from i64 to u64 failed.")]
    TimestampConversionError, // 0x1785 (6021)
    #[msg("Timestamp must be greater than the last updated timestamp.")]
    InvalidTimestampError, // 0x1786 (6022)
    #[msg("Invalid tick array sequence.")]
    InvalidTickArraySequenceError, // 0x1787 (6023)
    #[msg("Incorrect token mint order.")]
    InvalidTokenMintOrderError, // 0x1788 (6024)
    #[msg("Reward not initialized.")]
    RewardNotInitializedError, // 0x1789 (6025)
    #[msg("Invalid reward index.")]
    InvalidRewardIndexError, // 0x178a (6026)
    #[msg("Insufficient reward vault amount for emissions.")]
    InsufficientRewardVaultAmountError, // 0x178b (6027)
    #[msg("Exceeded maximum fee rate.")]
    FeeRateExceededError, // 0x178c (6028)
    #[msg("Exceeded maximum protocol fee rate.")]
    ProtocolFeeRateExceededError, // 0x178d (6029)
    #[msg("Multiplication with shift right overflow.")]
    MultiplicationShiftRightOverflowError, // 0x178e (6030)
    #[msg("MulDiv overflow error.")]
    MulDivOverflowError, // 0x178f (6031)
    #[msg("Invalid input for MulDiv.")]
    MulDivInvalidInputError, // 0x1790 (6032)
    #[msg("Multiplication overflow error.")]
    MultiplicationOverflowError, // 0x1791 (6033)
    #[msg("Invalid SqrtPriceLimit direction for swap.")]
    InvalidSqrtPriceLimitDirectionError, // 0x1792 (6034)
    #[msg("No tradable amount available for swap.")]
    NoTradableAmountError, // 0x1793 (6035)
    #[msg("Amount out is below the minimum threshold.")]
    AmountOutBelowMinimumError, // 0x1794 (6036)
    #[msg("Amount in exceeds the maximum threshold.")]
    AmountInAboveMaximumError, // 0x1795 (6037)
    #[msg("Invalid index for tick array sequence.")]
    InvalidTickArraySequenceErrorIndexError, // 0x1796 (6038)
    #[msg("Calculated amount overflows.")]
    AmountCalculationOverflowError, // 0x1797 (6039)
    #[msg("Remaining amount overflows.")]
    AmountRemainingOverflowError, // 0x1798 (6040)
    #[msg("Invalid intermediary mint.")]
    InvalidIntermediaryMintError, // 0x1799 (6041)
    #[msg("Duplicate two-hop pool.")]
    DuplicateTwoHopPoolError, // 0x179a (6042)
    #[msg("Trade batch index is out of bounds.")]
    InvalidTradeBatchIndexError, // 0x179b (6043)
    #[msg("Position has already been opened.")]
    PositionAlreadyOpenedError, // 0x179c (6044)
    #[msg("Position has already been closed.")]
    PositionAlreadyClosedError, // 0x179d (6045)
    #[msg("Cannot delete position trade batch with open positions.")]
    NonDeletablePositionTradeBatchError, // 0x179e (6046)
    #[msg("Unsupported token mint attributes.")]
    UnsupportedTokenMintError, // 0x179f (6047)
    #[msg("Invalid remaining accounts slice.")]
    InvalidRemainingAccountsSliceError, // 0x17a0 (6048)
    #[msg("Insufficient remaining accounts.")]
    InsufficientRemainingAccountsError, // 0x17a1 (6049)
    #[msg("Transfer hook requires extra accounts.")]
    MissingExtraAccountsForTransferHookError, // 0x17a2 (6050)
    #[msg("Mismatch between output and input amounts.")]
    AmountMismatchError, // 0x17a3 (6051)
    #[msg("Failed to calculate transfer fee.")]
    TransferFeeCalculationError, // 0x17a4 (6052)
    #[msg("Duplicate account types provided.")]
    DuplicateAccountTypesError, // 0x17a5 (6053)
    #[msg("Only full-range positions are supported in this pool.")]
    FullRangeOnlyPoolError, // 0x17a6 (6054)
    #[msg("The provided input token mint does not match the expected value.")]
    InvalidInputTokenMint, // 0x17a7 (6055)
    #[msg("The provided intermediate token mint does not match the expected value.")]
    InvalidIntermediateTokenMint, // 0x17a8 (6056)
    #[msg("The provided output token mint does not match the expected value.")]
    InvalidOutputTokenMint, // 0x17a9 (6057)
    #[msg("The vault provided does not match the expected vault.")]
    InvalidVault, // 0x17aa (6058)
    #[msg("The token owner account does not match the expected token mint.")]
    InvalidTokenOwner, // 0x17ab (6059)
    #[msg("The provided reward authority does not match the expected authority for the reward.")]
    InvalidRewardAuthorityError, // 0x17ac (6060)
    #[msg("The provided seed does not match the expected value.")]
    InvalidSeed, // 0x17ad (6061)
    #[msg("The provided reward mint does not match the expected value.")]
    InvalidRewardMintError, // 0x17ae (6062)
    #[msg("Token vaults have already been initialized.")]
    VaultAlreadyInitialized, // 0x17af (6063)
    #[msg("Super admin has already been initialized.")]
    SuperAdminAlreadyInitialized, // 0x17b0 (6064)
    #[msg("Invalid temporary pool funder.")]
    InvalidTemporaryPoolFunderError, // 0x17b1 (6065)
    #[msg("Invalid liquidity provision window.")]
    LiquidityProvisionWindowClosed, // 0x17b2 (6066)
    #[msg("Invalid swap window.")]
    SwapWindowClosed, // 0x17b3 (6067)
    #[msg("Pool with oracle should be only full range")]
    InvalidOraclePoolTickSpacing, // 0x17b4 (6068)
    #[msg("Missing oracle price feed id")]
    MissingOraclePriceFeedId, // 0x17b5 (6069)
    #[msg("Invalid oracle price")]
    InvalidPrice, // 0x17b6 (6070)
    #[msg("Missing oracle price update")]
    MissingPriceUpdate, // 0x17b7 (6071)
    #[msg("Missing initial sqrt price")]
    MissingInitialSqrtPrice, // 0x17b8 (6072)
    #[msg("Missing oracle account")]
    MissingOracleAccount, // 0x17b9 (6073)
    #[msg("Missing timestamps")]
    MissingTimestamps, // 0x17ba (6074)
    #[msg("Missing max age")]
    MissingMaxAge, // 0x17bb (6075)
    #[msg("Invalid price update account")]
    InvalidPriceUpdateAccount, // 0x17bc (6076)
    #[msg("Exceeded maximum referral reward fee rate")]
    ReferralRewardFeeRateExceededError, // 0x17bd (6077)
    #[msg("Missing swap referral account")]
    MissingSwapReferralAccount,
    #[msg("Missing swap referral ATA")]
    MissingSwapReferralAta,
    #[msg("Invalid swap referral ATA")]
    InvalidSwapReferralAta,
    #[msg("Reinvestment is not enabled for this position.")]
    ReinvestmentNotEnabled,
    #[msg("Invalid provided config account")]
    InvalidAiDexConfig,
}

impl From<TryFromIntError> for ErrorCode {
    fn from(_: TryFromIntError) -> Self {
        ErrorCode::BigIntCastError
    }
}
