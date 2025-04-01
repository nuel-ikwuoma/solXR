use crate::constants::{BOND_MATURITY, BOND_PRICE, DURATION, MAX_MINT_PER_WALLET, MINTING_ROUNDS, MIN_PREMIUM_NAV_RATIO, NAV_GROWTH_RATE, PLATFORM_MINT_FEE, GOVERNANCE_AUTHORITY,SOLXR_DECIMAL};
use crate::state::sol_strategy::SolStrategy;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        mpl_token_metadata::instructions::{
            CreateMasterEditionV3Cpi, CreateMasterEditionV3CpiAccounts,
            CreateMasterEditionV3InstructionArgs, CreateMetadataAccountV3Cpi,
            CreateMetadataAccountV3CpiAccounts, CreateMetadataAccountV3InstructionArgs,
        },
        mpl_token_metadata::types::{CollectionDetails, DataV2},
        CreateMetadataAccountsV3, Metadata,
    },
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};
use crate::PLATFORM_ADDRESS;

#[derive(Accounts)]
pub struct InitializeToken<'info> {
    #[account(
        mut,
        constraint = GOVERNANCE_AUTHORITY == governance_authority.key() @ Error::UnauthorizedGovernanceAuthority, // todo update to official controlled governance address
    )]
    pub governance_authority: Signer<'info>,

    #[account(
        init_if_needed,
        payer = governance_authority,
        space = 8 + SolStrategy::INIT_SPACE,
        seeds = [SolStrategy::SEED_PREFIX],
        bump
    )]
    pub sol_strategy: Account<'info, SolStrategy>,

    // freeze authority removed to enable trading
    #[account(
        init,
        payer = governance_authority,
        mint::decimals = SOLXR_DECIMAL,
        mint::authority = sol_strategy.key(),
        seeds = [b"token"],
        bump
    )]
    pub token: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), token.key().as_ref()],
        bump,
        seeds::program = metadata_program.key(),
    )]
    /// CHECK: Validated by PDA derivation
    pub token_metadata: UncheckedAccount<'info>,

    pub metadata_program: Program<'info, Metadata>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> InitializeToken<'info> {
    pub fn handler(
        &mut self,
        bumps: &InitializeTokenBumps,
        initial_pool_cap: u64,
        individual_address_cap: u64,
    ) -> Result<()> {
        self.sol_strategy.set_inner(SolStrategy {
            initial_pool_cap,
            individual_address_cap,
            sol_in_treasury: 0,
            governance_authority: self.governance_authority.key(), // todo update to official controlled governance address
            platform_address: PLATFORM_ADDRESS,
            allow_new_mint: false,
            platform_mint_fee: PLATFORM_MINT_FEE,
            max_mint_per_wallet: MAX_MINT_PER_WALLET,
            min_premium_nav_ratio: MIN_PREMIUM_NAV_RATIO,
            nav_growth_rate: NAV_GROWTH_RATE,
            minting_rounds: MINTING_ROUNDS,
            next_minting_rounds: 1,
            mint_duration: DURATION,
            bond_price: BOND_PRICE,
            bond_maturity: BOND_MATURITY,
        });

        let token_metadata = &self.token_metadata.to_account_info();
        let token_mint = &self.token.to_account_info();
        let authority = &self.sol_strategy.to_account_info();
        let payer = &self.governance_authority.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let metadata_program = &self.metadata_program.to_account_info();
        let rent = &self.rent.to_account_info();

        let mint_auth_bump = bumps.sol_strategy;
        let mint_auth_seeds: &[&[u8]] = &[SolStrategy::SEED_PREFIX, &[mint_auth_bump]];
        let mint_auth_signer: &[&[&[u8]]] = &[&mint_auth_seeds[..]];

        // Token Metadata
        let token_metadata_account = CreateMetadataAccountV3Cpi::new(
            metadata_program,
            CreateMetadataAccountV3CpiAccounts {
                mint: token_mint,
                metadata: token_metadata,
                payer,
                mint_authority: authority,
                update_authority: (authority, true),
                system_program,
                rent: Some(rent),
            },
            CreateMetadataAccountV3InstructionArgs {
                data: DataV2 {
                    name: "Solana Strategy Token".to_owned(),
                    symbol: "SOLXR".to_owned(),
                    uri: "https://bafybeiaozf4pmo62t6tqbe4d66yfilxssot37wiqtp4l7ilvy43jpnyp3a.ipfs.w3s.link/metadata.json".to_owned(),
                    seller_fee_basis_points: 0,
                    creators: None,
                    collection: None,
                    uses: None,
                },
                is_mutable: true,
                collection_details: None,
            },
        );
        token_metadata_account.invoke_signed(mint_auth_signer)?;

        Ok(())
    }
}

#[error_code]
pub enum Error {
    #[msg("The account that calls this function must match the token initializer.")]
    UnauthorizedGovernanceAuthority,
}
