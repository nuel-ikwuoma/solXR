use anchor_lang::prelude::*;
use anchor_lang::{account, InitSpace};


#[account]
#[derive(InitSpace)]
pub struct MintRound {
    pub premium: u64,
    pub max_mint_per_wallet: u64,
    pub solxr_minted: u64,
    pub solxr_available: u64,
    pub start: u64,
}

impl MintRound {
    pub const SEED_PREFIX: &'static [u8] = b"mint_round";
}

#[account]
#[derive(InitSpace)]
pub struct AssociatedRoundAccount {
    pub amount_minted: u64,
}