use {
    crate::{
        mint_round::{AssociatedRoundAccount, MintRound},
        state::sol_strategy::SolStrategy,
    },
    anchor_lang::prelude::Rent,
    anchor_lang::prelude::*,
    anchor_lang::system_program,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
    std::ops::{Sub},
};

#[derive(Accounts)]
#[instruction(id: u64,amount: u64)]
pub struct BuySolxr<'info> {
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
        mut,
        seeds = [b"treasury"],
        bump
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        mut,
        constraint = sol_strategy.allow_new_mint == true @ Error::MintingNotAllowed, // check if new mint is open
        constraint = sol_strategy.next_minting_rounds == id @ Error::InvalidMintingRound, // check if new mint is open
        constraint = Self::calculate_solxr_to_mint(amount, mint_round.premium) + mint_round.solxr_minted <= mint_round.solxr_available @ Error::ExceedsAvailableSolxr, // check if amount won't go over available mint
        constraint = amount + associated_round_account.amount_minted <= mint_round.max_mint_per_wallet @ Error::ExceedsMaxMintPerWallet, // check if user doesn't mint more than max
        constraint = Clock::get()?.unix_timestamp as u64 - mint_round.start <= sol_strategy.mint_duration  @ Error::MintingDurationEnded, // check if duration of minting is not over
        seeds = [MintRound::SEED_PREFIX,&id.to_le_bytes()],
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

    /// CHECK
    #[account(
        mut,
        constraint = platform_address.key() == sol_strategy.platform_address @ Error::InvalidPlatformAccount
    )]
    pub platform_address: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> BuySolxr<'info> {
    pub fn handler(&mut self, bumps: &BuySolxrBumps, _id: u64, amount: u64) -> Result<()> {
        let mut platform_fee =
            Self::calculate_platform_fee(self.sol_strategy.platform_mint_fee, amount);
        if platform_fee > self.sol_strategy.max_platform_mint_fee {
            platform_fee = self.sol_strategy.max_platform_mint_fee;
        }

        // Transfer SOL to platform designated account
        system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: self.investor.to_account_info(),
                    to: self.platform_address.to_account_info(),
                },
            ),
            platform_fee,
        )?;

        // Transfer SOL to treasury
        system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: self.investor.to_account_info(),
                    to: self.treasury.to_account_info(),
                },
            ),
            amount.sub(platform_fee),
        )?;
        let solxr_to_mint =
            Self::calculate_solxr_to_mint(amount - platform_fee, self.mint_round.premium);

        // Mint token for payer
        // Get the bump for the mint authority PDA
        let mint_auth_bump = bumps.sol_strategy;
        let mint_auth_seeds: &[&[u8]] = &[SolStrategy::SEED_PREFIX, &[mint_auth_bump]];
        let mint_auth_signer: &[&[&[u8]]] = &[&mint_auth_seeds[..]];

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
            solxr_to_mint,
        )?;

        self.mint_round.solxr_minted += solxr_to_mint;
        self.associated_round_account.amount_minted += amount;
        Ok(())
    }

    fn calculate_platform_fee(platform_mint_fee: u64, amount: u64) -> u64 {
        let fee = platform_mint_fee as u128 * amount as u128 / u128::pow(10, 9);
        fee as u64
    }
    fn calculate_solxr_to_mint(amount: u64, premium: u64) -> u64 {
        let value = amount as u128 * u128::pow(10, 9) / premium as u128;
        value as u64
    }
}

#[error_code]
enum Error {
    #[msg("The provided platform account doesn't match the one stored in the strategy")]
    InvalidPlatformAccount,
    #[msg("New minting is not allowed at this time")]
    MintingNotAllowed,
    #[msg("The minting round does not match the next minting round")]
    InvalidMintingRound,
    #[msg("Exceeds maximum mint amount per wallet")]
    ExceedsMaxMintPerWallet,
    #[msg("Exceeds available Solxr for minting in this round")]
    ExceedsAvailableSolxr,
    #[msg("Minting round duration has ended")]
    MintingDurationEnded,
}
