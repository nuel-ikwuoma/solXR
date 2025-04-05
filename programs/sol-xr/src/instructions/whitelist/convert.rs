use {
    crate::state::{
        sol_strategy::SolStrategy,
        whitelists::{Whitelist, WhitelistNFTRecord},
    },
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

#[derive(Accounts)]
#[instruction(id: u64,edition_number: u64)]
pub struct ConvertWhitelist<'info> {
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
        seeds = [Whitelist::SEED_PREFIX,&id.to_le_bytes()],
        bump
    )]
    pub whitelist: Account<'info, Whitelist>,

    #[account(
        init_if_needed,
        payer = buyer,
        space = 8 + WhitelistNFTRecord::INIT_SPACE,
        seeds = [WhitelistNFTRecord::SEED_PREFIX, whitelist.key().as_ref(), edition_number.to_le_bytes().as_ref()],
        bump
    )]
    pub whitelist_edition_record: Account<'info, WhitelistNFTRecord>,

    #[account(
        mut,
        seeds = [whitelist.key().as_ref(), edition_number.to_le_bytes().as_ref()],
        bump
    )]
    pub buyer_whitelist_nft: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = buyer_whitelist_nft,
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

impl<'info> ConvertWhitelist<'info> {
    pub fn handler(
        &mut self,
        bumps: &ConvertWhitelistBumps,
        _id: u64,
        _edition_number: u64,
    ) -> Result<()> {
        let whitelist = &self.whitelist;
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp as u64;

        require!(
            !self.whitelist_edition_record.converted,
            Error::WhitelistClaimed
        );
        require!(
            current_time >= whitelist.maturity,
            Error::WhitelistNotMatured
        );
        require!(
            current_time <= whitelist.expiration,
            Error::WhitelistExpired
        );

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
            whitelist.price,
        )?;

        self.whitelist_edition_record.converted = true;
        Ok(())
    }
}

#[error_code]
enum Error {
    #[msg("The whitelist has already been claimed.")]
    WhitelistClaimed,
    #[msg("The whitelist has not yet matured.")]
    WhitelistNotMatured,
    #[msg("You can no longer convert your NFT.")]
    WhitelistExpired,
    #[msg("The token account must contain exactly 1 NFT.")]
    InvalidTokenAmount,
}
