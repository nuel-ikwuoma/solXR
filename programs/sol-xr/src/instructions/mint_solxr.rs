use crate::mint_round::{AssociatedRoundAccount, MintRound};
use crate::{Invest, InvestBumps};
use anchor_spl::metadata::Metadata;
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
#[instruction(id: u64,amount: u64)]
pub struct MintSolXR<'info> {
    #[account(mut)]
    pub investor: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token"],
        bump
    )]
    pub token: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = investor,
        associated_token::mint = token,
        associated_token::authority = investor,
    )]
    pub associated_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [SolStrategy::SEED_PREFIX],
        bump
    )]
    pub sol_strategy: Account<'info, SolStrategy>,

    #[account(
        constraint = sol_strategy.allow_new_mint == true, // check if new mint is open
        constraint = sol_strategy.next_minting_rounds == id, // check if round id matches the current round id
        constraint = sol_strategy.next_minting_rounds == sol_strategy.minting_rounds, //  check if minting rounds is not over
        constraint = amount + associated_round_account.amount_minted <= mint_round.max_mint_per_wallet, // check if user doesn't mint more than max
        constraint = amount + mint_round.solxr_minted <= mint_round.solxr_available, // check if amount won't go over available mint
        constraint = mint_round.start + sol_strategy.duration < Clock::get()?.unix_timestamp as u64, // check if duration of minting is not over
        seeds = [&id.to_le_bytes(),MintRound::SEED_PREFIX],
        bump
    )]
    pub mint_round: Account<'info, MintRound>,

    #[account(
        init_if_needed,
        payer = investor,
        space = 8 + AssociatedRoundAccount::INIT_SPACE,
        seeds = [mint_round.key().as_ref(), investor.key().as_ref()],
        bump
    )]
    pub associated_round_account: Account<'info, AssociatedRoundAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> MintSolXR<'info> {
    pub fn handler(&mut self, bumps: &MintSolXRBumps, id: u64,amount: u64) -> Result<()> {
        // todo: remove and transfer fee to designated account
        // todo: calculate sol amount left after fee and send too treasury
        // todo: get solxr to mint i.e. sol amount left /  market value of solXR to sol
        // todo: mint solxr to investor
        // todo: update solxr minted in MintRound
        // todo: update AssociatedRoundAccount amount_minted
        Ok(())
    }
}
