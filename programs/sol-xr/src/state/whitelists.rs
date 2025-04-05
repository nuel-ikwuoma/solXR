use anchor_lang::prelude::*;
use anchor_lang::{account, InitSpace};

#[account]
#[derive(InitSpace)]
pub struct Whitelist {
    pub maturity: u64,
    pub expiration: u64,
    pub price: u64,
    pub max_mint_per_wallet: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub next_edition_number: u64,
    #[max_len(248)]
    pub next_edition_marker: String,
}

impl Whitelist {
    pub const SEED_PREFIX: &'static [u8] = b"whitelist";
}

#[account]
#[derive(InitSpace)]
pub struct WhitelistRecord {
    pub collection: Pubkey,
    pub user: Pubkey,
    pub minted: u64,
}
impl WhitelistRecord {
    pub const SEED_PREFIX: &'static [u8] = b"whitelist_record";
}

#[account]
#[derive(InitSpace)]
pub struct WhitelistNFTRecord {
    pub converted: bool,
}
impl WhitelistNFTRecord {
    pub const SEED_PREFIX: &'static [u8] = b"whitelist_nft_record";
}