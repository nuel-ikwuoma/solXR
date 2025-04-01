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
#[instruction(id: u64,market_value: u64)]
pub struct OpenMintingRound<'info> {
    #[account(
        mut,
        constraint = sol_strategy.governance_authority.key() == governance_authority.key() @ Error::UnauthorizedGovernanceAuthority,
    )]
    pub governance_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"token"],
        bump
    )]
    pub token: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [SolStrategy::SEED_PREFIX],
        bump
    )]
    pub sol_strategy: Account<'info, SolStrategy>,

    #[account(
        init_if_needed,
        payer = governance_authority,
        constraint = sol_strategy.allow_new_mint == false @ Error::MintingAlreadyAllowed,
        constraint = sol_strategy.next_minting_rounds == id @ Error::IncorrectRoundId,
        constraint = sol_strategy.next_minting_rounds <= sol_strategy.minting_rounds @ Error::AllMintRoundsCompleted, // todo: test after multiple round creation
        constraint = token.supply > 0 @ Error::TokenSupplyIsZero,
        space = 8 + MintRound::INIT_SPACE,
        seeds = [MintRound::SEED_PREFIX,&id.to_le_bytes()],
        bump
    )]
    pub mint_round: Account<'info, MintRound>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> OpenMintingRound<'info> {
    pub fn handler(
        &mut self,
        bumps: &OpenMintingRoundBumps,
        id: u64,
        market_value: u64,
    ) -> Result<()> {
        let pass_mint_value_requirement = Self::check_mint_value_requirement(
            market_value,
            self.sol_strategy.min_premium_nav_ratio,
            self.sol_strategy.sol_in_treasury,
            self.token.supply,
        )?;
        require!(pass_mint_value_requirement, Error::MarketValueBelowMinPremium);

        self.sol_strategy.allow_new_mint = true;

        self.mint_round.premium = market_value;
        self.mint_round.max_mint_per_wallet = self.sol_strategy.max_mint_per_wallet;
        self.mint_round.solxr_minted = 0;
        self.mint_round.solxr_available = Self::calculate_solxr_to_mint(
            self.sol_strategy.nav_growth_rate,
            self.token.supply,
            market_value,
            self.sol_strategy.sol_in_treasury,
        )?;
        self.mint_round.start = Clock::get()?.unix_timestamp as u64;
        Ok(())
    }

    fn calculate_solxr_to_mint(
        nav_growth_rate: u64,
        solxr_supply: u64,
        market_value: u64,
        sol_in_treasury_lamports: u64,
    ) -> Result<u64> {
        let nav_growth_rate_decimal = (nav_growth_rate as u128) * (u128::pow(10, 9)) / 1_000_000_000;
        let nav = (sol_in_treasury_lamports as u128) * (u128::pow(10, 9)) / (solxr_supply as u128);
        require!(nav != 0, Error::NavIsZero);
        let market_value_ratio = (market_value as u128) * (u128::pow(10, 9)) / (nav as u128);
        require!(
        market_value_ratio > (u128::pow(10, 9) + nav_growth_rate_decimal),
        Error::MarketValueInsufficientForFormula
    );
        let denominator = market_value_ratio - u128::pow(10, 9) - nav_growth_rate_decimal;
        require!(denominator != 0, Error::MarketValueAtTargetNav); // todo: test market value at target nav
        let numerator = nav_growth_rate_decimal * (solxr_supply as u128);

        let result = numerator / denominator;
        require!(result <= u64::MAX as u128, Error::CalculatedMintableOverflow);
        Ok(result as u64)
    }

    fn check_mint_value_requirement(
        market_value: u64,
        min_premium_nav_ratio: u64,
        sol_in_treasury_lamports: u64,
        token_supply: u64,
    ) -> Result<bool> {
        let min_premium_decimal = (min_premium_nav_ratio as u128) * (u128::pow(10, 9)) / 1_000_000_000;
        require!(token_supply != 0, Error::TokenSupplyIsZero);
        let nav = (sol_in_treasury_lamports as u128) * (u128::pow(10, 9)) / (token_supply as u128);
        let min_required_value = (min_premium_decimal + u128::pow(10, 9)) * nav / u128::pow(10, 9);
        let result = market_value as u128 >= min_required_value;
        Ok(result)
    }
}


#[error_code]
pub enum Error {
    #[msg("Caller is not the required governance authority defined in the SolStrategy.")]
    UnauthorizedGovernanceAuthority,

    #[msg("Cannot open a new round because `allow_new_mint` is already true in SolStrategy.")]
    MintingAlreadyAllowed,

    #[msg("The provided round ID does not match the expected `next_minting_rounds` in SolStrategy.")]
    IncorrectRoundId,

    #[msg("Cannot open round: The next round ID exceeds the total number of rounds planned.")]
    AllMintRoundsCompleted,

    #[msg("The provided market value does not meet the minimum required premium over NAV.")]
    MarketValueBelowMinPremium,

    #[msg("Calculation failed: Net Asset Value (NAV) per token is zero.")]
    NavIsZero,

    #[msg("Cannot proceed with zero token supply; required for NAV and premium calculations.")]
    TokenSupplyIsZero,

    #[msg("Calculation failed: Market value equals target NAV threshold, resulting in zero denominator.")]
    MarketValueAtTargetNav,

    #[msg("Calculation failed: Resulting SOLXR available to mint exceeds maximum value (u64::MAX).")]
    CalculatedMintableOverflow,

    #[msg("Calculation failed: Market value to NAV ratio must exceed (1 + NAV growth rate) for formula.")]
    MarketValueInsufficientForFormula,
}