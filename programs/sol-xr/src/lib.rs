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
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        initial_pool_cap: u64,
        individual_address_cap: u64,
    ) -> Result<()> {
        initialize_handler(ctx, initial_pool_cap, individual_address_cap)
    }

    pub fn invest(ctx: Context<Invest>, amount: u64) -> Result<()> {
        invest_handler(ctx, amount)
    }
}
