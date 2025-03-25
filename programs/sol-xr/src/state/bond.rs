use anchor_lang::prelude::*;
use anchor_lang::{account, InitSpace};

#[account]
#[derive(InitSpace)]
pub struct Bond {
    pub maturity: u64,
    pub strike_price: u64,
}

impl Bond {
    pub const SEED_PREFIX: &'static [u8; 4] = b"bond";
}
