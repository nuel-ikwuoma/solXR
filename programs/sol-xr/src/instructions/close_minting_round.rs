use crate::mint_round::{AssociatedRoundAccount, MintRound};
use crate::{Invest, InvestBumps};
use anchor_spl::metadata::Metadata;
use std::ops::Div;
use {
    crate::state::sol_strategy::SolStrategy,
    anchor_lang::prelude::Rent,
    anchor_lang::prelude::*,
    anchor_lang::system_program,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

#[derive(Accounts)]
pub struct CloseMintingRound<'info> {
    #[account(
        mut,
        constraint = sol_strategy.governance_authority.key() == governance_authority.key() @ Error::UnauthorizedGovernanceAuthority,
    )]
    pub governance_authority: Signer<'info>,
    #[account(
        mut,
        seeds = [SolStrategy::SEED_PREFIX],
        bump
    )]
    pub sol_strategy: Account<'info, SolStrategy>,
    #[account(
        init_if_needed,
        payer = governance_authority,
        constraint = sol_strategy.allow_new_mint == true @ Error::MintingAlreadyClosed,
        space = 8 + MintRound::INIT_SPACE,
        seeds = [MintRound::SEED_PREFIX,&sol_strategy.next_minting_rounds.to_le_bytes()],
        bump
    )]
    pub mint_round: Account<'info, MintRound>,
    pub system_program: Program<'info, System>,
}

impl<'info> CloseMintingRound<'info> {
    pub fn handler(&mut self, bumps: &CloseMintingRoundBumps) -> Result<()> {
        self.sol_strategy.allow_new_mint = false;
        self.sol_strategy.next_minting_rounds += 1;
        Ok(())
    }
}

#[error_code]
pub enum Error {
    #[msg("Caller is not the required governance authority defined in the SolStrategy.")]
    UnauthorizedGovernanceAuthority,

    #[msg("Cannot close round because `allow_new_mint` is already false in SolStrategy.")]
    MintingAlreadyClosed,
}
