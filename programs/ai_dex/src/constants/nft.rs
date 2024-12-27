use anchor_lang::prelude::*;

pub mod ai_dex_nft_update_auth {
    use super::*;
    declare_id!("updmeGm2r24F2USBMiscZEZr89nxyy2LvmpQwUAmzjD");
}

// METADATA_NAME   : max  32 bytes
pub const AD_METADATA_NAME: &str = "Ai Dex Position";
// METADATA_SYMBOL : max  10 bytes
pub const AD_METADATA_SYMBOL: &str = "ADP";
// METADATA_URI    : max 200 bytes
pub const AD_METADATA_URI: &str = "https://ipfs.io/ipfs/QmWwbhFVsLfrP5TYSKV37g7fNVtucEg999bGNLmMWYHHr2";

// pub const ADB_METADATA_NAME_PREFIX: &str = "Ai Dex Position TradeBatch";
pub const ADB_METADATA_SYMBOL: &str = "ADPB";
pub const ADB_METADATA_URI: &str =
    "https://ipfs.io/ipfs/QmXDMmmCVZMcDzwtMmqZt6uKtKSeoGaW4ajVxzz7nrUrWG";
