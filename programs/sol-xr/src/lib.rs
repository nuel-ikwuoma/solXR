pub mod constants;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use state::*;
pub use instructions::*;

declare_id!("2oAJBBNEGWnxbH65MEWuehjjmbN6Gk9uLiK9Wt6cR3cT");

#[program]
pub mod sol_xr {
    use super::*;

    pub fn initialize_token(
        ctx: Context<InitializeToken>,
        initial_pool_cap: u64,
        individual_address_cap: u64,
    ) -> Result<()> {
        ctx.accounts
            .handler(&ctx.bumps, initial_pool_cap, individual_address_cap)
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
    pub fn close_mint_round(ctx: Context<CloseMintingRound>) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps)
    }
    pub fn buy_solxr(ctx: Context<BuySolxr>, id: u64, amount: u64) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps, id, amount)
    }

    pub fn sell_bond(
        ctx: Context<SellBond>,
        name: String,
        symbol: String,
        uri: String,
        maturity: u64,
        strike_price: u64,
        supply: u64,
        price: u64,
        max_mint_per_wallet: u64,
        start_time: u64,
        end_time: u64,
    ) -> Result<()> {
        ctx.accounts.handler(
            &ctx.bumps,
            name,
            symbol,
            uri,
            maturity,
            strike_price,
            supply,
            price,
            max_mint_per_wallet,
            start_time,
            end_time,
        )
    }
    pub fn buy_bond(ctx: Context<BuyBond>, id: u64) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps, id)
    }
    pub fn convert_bond(
        ctx: Context<ConvertBond>,
        id: u64,
        edition_number: u64,
        convert: bool,
    ) -> Result<()> {
        ctx.accounts
            .handler(&ctx.bumps, id, edition_number, convert)
    }
}
