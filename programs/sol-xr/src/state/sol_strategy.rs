use anchor_lang::prelude::*;
use anchor_lang::{account, InitSpace};

#[account]
#[derive(InitSpace)]
pub struct SolStrategy {
    pub initial_pool_cap: u64,
    pub individual_address_cap: u64,
    pub bond_price: u64,
    pub sol_in_pool: u64,
}

impl SolStrategy {
    pub const SEED_PREFIX: &'static [u8; 12] = b"sol_strategy";
}
