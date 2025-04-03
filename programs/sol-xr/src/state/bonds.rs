use anchor_lang::prelude::*;
use anchor_lang::{account, InitSpace};

#[account]
#[derive(InitSpace)]
pub struct Bond {
    pub maturity: u64,
    pub strike_price: u64,
    pub supply: u64,
    pub price: u64,
    pub max_mint_per_wallet: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub next_edition_number: u64,
}

impl Bond {
    pub const SEED_PREFIX: &'static [u8] = b"bond";
}

#[account]
#[derive(InitSpace)]
pub struct BondRecord {
    pub collection: Pubkey,
    pub user: Pubkey,
    pub minted: u64,
}
impl BondRecord {
    pub const SEED_PREFIX: &'static [u8] = b"bond_record";
}