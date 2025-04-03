use {
    crate::{
        state::{bond::Bond, sol_strategy::SolStrategy},
    },
    anchor_lang::prelude::*,
    anchor_spl::{
        associated_token::AssociatedToken,
        metadata::{
            mpl_token_metadata::{
                instructions::{
                    CreateMasterEditionV3Cpi, CreateMasterEditionV3CpiAccounts,
                    CreateMasterEditionV3InstructionArgs, CreateMetadataAccountV3Cpi,
                    CreateMetadataAccountV3CpiAccounts, CreateMetadataAccountV3InstructionArgs,
                },
                types::{CollectionDetails, DataV2},
            },
            Metadata,
        },
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

#[derive(Accounts)]
pub struct SellBond<'info> {
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
        init,
        payer = governance_authority,
        space = 8 + Bond::INIT_SPACE,
        seeds = [Bond::SEED_PREFIX,sol_strategy.next_bond_id.to_le_bytes().as_ref()],
        bump
    )]
    pub bond: Account<'info, Bond>,

    #[account(
        init,
        payer = governance_authority,
        mint::decimals = 0,
        mint::authority = sol_strategy,
        mint::freeze_authority = sol_strategy,
        seeds = [bond.key().as_ref()],
        bump
    )]
    pub bond_nft: Account<'info, Mint>,

    #[account(
        init,
        payer = governance_authority,
        associated_token::mint = bond_nft,
        associated_token::authority = sol_strategy,
    )]
    pub bond_token_account: Account<'info, TokenAccount>,
    /// CHECK: Validated by PDA derivation
    #[account(mut)]
    pub bond_metadata: UncheckedAccount<'info>,
    /// CHECK: Validated by PDA derivation
    #[account(mut)]
    pub bond_edition: UncheckedAccount<'info>,

    pub metadata_program: Program<'info, Metadata>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> SellBond<'info> {
    pub fn handler(
        &mut self,
        bumps: &SellBondBumps,
        name: String,
        symbol: String,
        uri: String,
        maturity: u64,
        strike_price: u64,
        supply: u64,
        price: u64,
        max_mint_per_wallet: u64,
        start_time: u64,
        end_time: u64,
    ) -> Result<()> {
        self.bond.set_inner(Bond {
            maturity,
            strike_price,
            supply,
            price,
            max_mint_per_wallet,
            start_time,
            end_time,
            next_edition_number: 1,
        });

        let mint_auth_bump = bumps.sol_strategy;
        let mint_auth_seeds: &[&[u8]] = &[SolStrategy::SEED_PREFIX, &[mint_auth_bump]];
        let mint_auth_signer: &[&[&[u8]]] = &[&mint_auth_seeds[..]];

        let nft_metadata = &self.bond_metadata.to_account_info();
        let master_edition = &self.bond_edition.to_account_info();
        let bond = &self.bond.to_account_info();
        let authority = &self.sol_strategy.to_account_info();
        let payer = &self.governance_authority.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let token_program = &self.token_program.to_account_info();
        let metadata_program = &self.metadata_program.to_account_info();
        let rent = &self.rent.to_account_info();

        // Mint 1 NFT token
        let mint_cpi = CpiContext::new_with_signer(
            token_program.clone(),
            MintTo {
                mint: bond.clone(),
                to: self.bond_token_account.to_account_info(),
                authority: authority.clone(),
            },
            mint_auth_signer,
        );
        mint_to(mint_cpi, 1)?;

        // NFT Metadata
        let nft_metadata_account = CreateMetadataAccountV3Cpi::new(
            metadata_program,
            CreateMetadataAccountV3CpiAccounts {
                metadata: nft_metadata,
                mint: bond,
                mint_authority: authority,
                payer,
                update_authority: (authority, true),
                system_program,
                rent: Some(rent),
            },
            CreateMetadataAccountV3InstructionArgs {
                data: DataV2 {
                    name,
                    symbol,
                    uri,
                    seller_fee_basis_points: 0,
                    creators: None,
                    collection: None,
                    uses: None,
                },
                is_mutable: true,
                collection_details: Some(CollectionDetails::V1 { size: 0 }),
            },
        );
        nft_metadata_account.invoke_signed(mint_auth_signer)?;

        // Create Master Edition
        let master_edition_account = CreateMasterEditionV3Cpi::new(
            metadata_program,
            CreateMasterEditionV3CpiAccounts {
                edition: master_edition,
                update_authority: authority,
                mint_authority: authority,
                mint: bond,
                payer,
                metadata: nft_metadata,
                token_program,
                system_program,
                rent: Some(rent),
            },
            CreateMasterEditionV3InstructionArgs {
                max_supply: Some(supply),
            }, // todo: test that supply is limited
        );
        master_edition_account.invoke_signed(mint_auth_signer)?;

        self.sol_strategy.next_bond_id += 1;

        Ok(())
    }
}

#[error_code]
enum Error {
    #[msg("Caller is not the required governance authority defined in the SolStrategy.")]
    UnauthorizedGovernanceAuthority,
}
