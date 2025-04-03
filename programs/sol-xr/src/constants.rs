use anchor_lang::prelude::*;

#[constant]
pub const SOLXR_DECIMAL: u8 = 9; // same as solana
pub const GOVERNANCE_AUTHORITY: Pubkey = pubkey!("DEvurheakNvpZQAASK5Fug9LbPSMzmT7BmwAMkuiGXsU");
pub const PLATFORM_ADDRESS: Pubkey = pubkey!("DEvurheakNvpZQAASK5Fug9LbPSMzmT7BmwAMkuiGXsU");
pub const PLATFORM_MINT_FEE: u64 = 30_000_000; // 3%
pub const MAX_PLATFORM_MINT_FEE: u64 = 500_000_000; // 0.5 sol
pub const MAX_MINT_PER_WALLET: u64 = 10_000_000_000; // 10 sol
pub const MIN_PREMIUM_NAV_RATIO: u64 = 500_000_000; // 50% where 100% = 1 sol (1_000_000_000 lamport)
pub const NAV_GROWTH_RATE: u64 = 100_000_000; // 10% where 100% = 1 sol (1_000_000_000 lamport)
pub const MINTING_ROUNDS: u64 = 24; // 6 months
pub const DURATION: u64 = 21600; // 6 hours
pub const BOND_PRICE: u64 = 1_000_000_000; // 1 sol
pub const BOND_MATURITY: u64 = 15_552_000; //  approx 6 months
