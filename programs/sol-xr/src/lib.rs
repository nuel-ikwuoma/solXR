pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("2oAJBBNEGWnxbH65MEWuehjjmbN6Gk9uLiK9Wt6cR3cT");

#[program]
pub mod sol_xr {
    use anchor_spl::token::accessor::amount;
    use crate::mint_solxr::MintSolXR;
    use super::*;

    pub fn initialize_token(
        ctx: Context<InitializeToken>,
        initial_pool_cap: u64,
        individual_address_cap: u64,
    ) -> Result<()> {
        ctx.accounts
            .handler(&ctx.bumps, initial_pool_cap, individual_address_cap)
    }
    pub fn initialize_nft(ctx: Context<InitializeNFT>, bond_price: u64) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps, bond_price)
    }

    pub fn invest(ctx: Context<Invest>, amount: u64) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps, amount)
    }
    pub fn open_mint_round(
        ctx: Context<OpenMintingRound>,
        id: u64,
        market_value: u64,
    ) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps, id, market_value)
    }
    pub fn close_mint_round(
        ctx: Context<CloseMintingRound>,
    ) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps)
    }
    pub fn mint_solxr(
        ctx: Context<MintSolXR>,
        id: u64,
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps, id, amount)
    }
}
