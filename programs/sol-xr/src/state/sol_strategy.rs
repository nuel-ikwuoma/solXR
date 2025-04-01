use anchor_lang::prelude::*;
use anchor_lang::{account, InitSpace};

#[account]
#[derive(InitSpace)]
pub struct SolStrategy {
    pub initial_pool_cap: u64,
    pub individual_address_cap: u64,
    /// Number of Sol in the treasury
    pub sol_in_treasury: u64,

    // Governance-controlled parameters
    /// An account with governance authority responsible for executing all governance-controlled instructions
    pub governance_authority: Pubkey,

    /// Designated Account for platform
    pub platform_address: Pubkey,

    // Minting New Shares
    /// Allow SolXR to be minted
    pub allow_new_mint: bool,
    /// Fee charged for every mint in lamport i.e. 1 sol (1_000_000_000 lamport) = 100%
    pub platform_mint_fee: u64,
    /// Maximum number of mint per account in lamport
    pub max_mint_per_wallet: u64,
    /// Minimum Premium to NAV ratio required before additional SolXR can be minted in lamport (e.g., 1 sol (1_000_000_000 lamport) = 100%, meaning if NAV is 1_000_000_000 lamport, the premium must be 2_000_000_000 lamport)
    pub min_premium_nav_ratio: u64,
    /// The percentage increase in Net Asset Value (NAV) expected from the mint operation in lamport i.e. 1 sol (1_000_000_000 lamport) = 100%
    pub nav_growth_rate: u64,
    /// Number of minting rounds
    pub minting_rounds: u64,
    /// Number of minting rounds
    pub next_minting_rounds: u64,
    /// Duration for minting
    pub mint_duration: u64,


    // Buying convertible bonds
    /// Price of convertible bonds in lamport
    pub bond_price: u64,
    /// Time until bond maturity, if set to 30 days, the bond will mature 30 days after issuance
    pub bond_maturity: u64,
}

impl SolStrategy {
    pub const SEED_PREFIX: &'static [u8] = b"sol_strategy";
}
