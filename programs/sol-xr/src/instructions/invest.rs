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
#[instruction(amount: u64)]
pub struct Invest<'info> {
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
        constraint = amount + sol_strategy.sol_in_treasury <= sol_strategy.initial_pool_cap @ Error::InitialSolCapError,
        constraint = amount + associated_token_account.amount <= sol_strategy.individual_address_cap @ Error::ATACapError,
        seeds = [SolStrategy::SEED_PREFIX],
        bump
    )]
    pub sol_strategy: Account<'info, SolStrategy>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Invest<'info> {
    pub fn handler(&mut self, bumps: &InvestBumps, amount: u64) -> Result<()> {
        // Get the bump for the mint authority PDA
        let mint_auth_bump = bumps.sol_strategy;
        let mint_auth_seeds: &[&[u8]] = &[SolStrategy::SEED_PREFIX, &[mint_auth_bump]];
        let mint_auth_signer: &[&[&[u8]]] = &[&mint_auth_seeds[..]];

        // Transfer SOL to treasury
        system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: self.investor.to_account_info(),
                    to: self.sol_strategy.to_account_info(),
                },
            ),
            amount,
        )?;

        // Mint token for payer
        mint_to(
            CpiContext::new(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.token.to_account_info(),
                    to: self.associated_token_account.to_account_info(),
                    authority: self.sol_strategy.to_account_info(),
                },
            )
            .with_signer(mint_auth_signer),
            amount, // Since solxr and sol have the same decimals
        )?;

        self.sol_strategy.sol_in_treasury += amount;

        Ok(())
    }
}

#[error_code]
enum Error {
    #[msg("The amount would cause the ATA balance to exceed the individual address cap.")]
    ATACapError,
    #[msg("The amount would cause the program PDA to exceed the initial pool cap.")]
    InitialSolCapError,
}
