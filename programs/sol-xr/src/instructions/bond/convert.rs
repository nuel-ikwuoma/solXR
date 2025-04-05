use {
    crate::state::{bonds::Bond, sol_strategy::SolStrategy},
    anchor_lang::prelude::*,
    anchor_lang::system_program,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{mint_to, Burn, Mint, MintTo, Token, TokenAccount,burn},
    },
};
#[derive(Accounts)]
#[instruction(id: u64,edition_number: u64)]
pub struct ConvertBond<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [SolStrategy::SEED_PREFIX],
        bump
    )]
    pub sol_strategy: Account<'info, SolStrategy>,

    #[account(
        mut,
        seeds = [b"treasury"],
        bump
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [Bond::SEED_PREFIX,&id.to_le_bytes()],
        bump
    )]
    pub bond: Account<'info, Bond>,

    #[account(
        mut,
        seeds = [bond.key().as_ref(), edition_number.to_le_bytes().as_ref()],
        bump
    )]
    pub buyer_bond_nft: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = buyer_bond_nft,
        associated_token::authority = buyer,
        constraint = associated_nft_account.amount == 1 @ Error::InvalidTokenAmount
    )]
    pub associated_nft_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"token"],
        bump
    )]
    pub token: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = token,
        associated_token::authority = buyer,
    )]
    pub associated_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> ConvertBond<'info> {
    pub fn handler(
        &mut self,
        bumps: &ConvertBondBumps,
        _id: u64,
        _edition_number: u64,
        convert: bool,
    ) -> Result<()> {
        let bond = &self.bond;
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        require!(current_time >= bond.maturity, Error::BondNotMatured);

        let burn_cpi_accounts = Burn {
            mint: self.buyer_bond_nft.to_account_info(),
            authority: self.buyer.to_account_info(),
            from: self.associated_nft_account.to_account_info(),
        };
        let burn_cpi_ctx = CpiContext::new(self.token_program.to_account_info(), burn_cpi_accounts);
        burn(burn_cpi_ctx, 1)?;


        if convert {
            let solxr_to_mint = Self::calculate_solxr_to_mint(bond.price, bond.strike_price);

            let sol_strategy_bump = bumps.sol_strategy;
            let sol_strategy_seeds: &[&[u8]] = &[SolStrategy::SEED_PREFIX, &[sol_strategy_bump]];
            let signer_seeds: &[&[&[u8]]] = &[&sol_strategy_seeds[..]];

            mint_to(
                CpiContext::new(
                    self.token_program.to_account_info(),
                    MintTo {
                        mint: self.token.to_account_info(),
                        to: self.associated_token_account.to_account_info(),
                        authority: self.sol_strategy.to_account_info(),
                    },
                )
                .with_signer(signer_seeds),
                solxr_to_mint,
            )?;

            self.sol_strategy.sol_from_bond -= bond.price;
            self.sol_strategy.sol_in_treasury += bond.price;
        } else {
            let sol_strategy_bump = bumps.treasury;
            let sol_strategy_seeds: &[&[u8]] = &[b"treasury", &[sol_strategy_bump]];
            let signer_seeds: &[&[&[u8]]] = &[&sol_strategy_seeds[..]];

            system_program::transfer(
                CpiContext::new_with_signer(
                    self.system_program.to_account_info(),
                    system_program::Transfer {
                        from: self.treasury.to_account_info(),
                        to: self.buyer.to_account_info(),
                    },
                    signer_seeds,
                ),
                bond.price,
            )?;

            self.sol_strategy.sol_from_bond -= bond.price;
        }

        Ok(())
    }

    fn calculate_solxr_to_mint(sol_amount: u64, strike_price: u64) -> u64 {
        let solxr_to_mint = sol_amount as u128 * u128::pow(10, 9) / strike_price as u128;
        solxr_to_mint as u64
    }
}

#[error_code]
enum Error {
    #[msg("The bond has not yet matured.")]
    BondNotMatured,
    #[msg("The token account must contain exactly 1 NFT.")]
    InvalidTokenAmount,
}
