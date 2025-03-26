use anchor_lang::prelude::*;

#[constant]
pub const MINT_AUTHORITY_SEED_PREFIX: &'static [u8; 5] = b"solxr";
pub const SOLXR_DECIMAL: u8 = 9;

pub const TOKEN_INITIALIZER: Pubkey = pubkey!("DEvurheakNvpZQAASK5Fug9LbPSMzmT7BmwAMkuiGXsU");
pub const NFT_INITIALIZER: Pubkey = pubkey!("DEvurheakNvpZQAASK5Fug9LbPSMzmT7BmwAMkuiGXsU");