use anchor_lang::prelude::*;
use anchor_lang::{account, InitSpace};

#[account]
#[derive(InitSpace)]
pub struct SolStrategy {
    pub initial_pool_cap: u64,
    pub current_sol_balance: u64,   // todo: fetch from mint PDA balance
    pub current_solxr_balance: u64, // todo: fetch from mint supply
}

impl SolStrategy {
    pub const SEED_PREFIX: &'static [u8; 12] = b"sol_strategy";
}
