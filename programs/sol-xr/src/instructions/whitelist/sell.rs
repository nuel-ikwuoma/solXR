use {
    crate::state::{whitelists::Whitelist, sol_strategy::SolStrategy},
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
pub struct SellWhitelist<'info> {
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
        space = 8 + Whitelist::INIT_SPACE,
        seeds = [Whitelist::SEED_PREFIX,sol_strategy.next_whitelist_id.to_le_bytes().as_ref()],
        bump
    )]
    pub whitelist: Account<'info, Whitelist>,

    #[account(
        init,
        payer = governance_authority,
        mint::decimals = 0,
        mint::authority = sol_strategy,
        mint::freeze_authority = sol_strategy,
        seeds = [whitelist.key().as_ref()],
        bump
    )]
    pub whitelist_nft: Account<'info, Mint>,

    #[account(
        init,
        payer = governance_authority,
        associated_token::mint = whitelist_nft,
        associated_token::authority = sol_strategy,
    )]
    pub whitelist_token_account: Account<'info, TokenAccount>,
    /// CHECK: Validated by PDA derivation
    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), whitelist_nft.key().as_ref()],
        bump,
        seeds::program = metadata_program.key(),
    )]
    pub whitelist_metadata: UncheckedAccount<'info>,
    /// CHECK: Validated by PDA derivation
    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), whitelist_nft.key().as_ref(),b"edition"],
        bump,
        seeds::program = metadata_program.key(),
    )]
    pub whitelist_edition: UncheckedAccount<'info>,

    pub metadata_program: Program<'info, Metadata>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> SellWhitelist<'info> {
    pub fn handler(
        &mut self,
        bumps: &SellWhitelistBumps,
        name: String,
        symbol: String,
        uri: String,
        price: u64,
        maturity: u64,
        expiration: u64,
        max_mint_per_wallet: u64,
        start_time: u64,
        end_time: u64,
    ) -> Result<()> {
        self.whitelist.set_inner(Whitelist {
            maturity,
            expiration,
            price,
            max_mint_per_wallet,
            start_time,
            end_time,
            next_edition_number: 1u64,
            next_edition_marker: (1 / 248).to_string(),
        });

        let mint_auth_bump = bumps.sol_strategy;
        let mint_auth_seeds: &[&[u8]] = &[SolStrategy::SEED_PREFIX, &[mint_auth_bump]];
        let mint_auth_signer: &[&[&[u8]]] = &[&mint_auth_seeds[..]];

        let nft_metadata = &self.whitelist_metadata.to_account_info();
        let master_edition = &self.whitelist_edition.to_account_info();
        let whitelist_nft = &self.whitelist_nft.to_account_info();
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
                mint: whitelist_nft.clone(),
                to: self.whitelist_token_account.to_account_info(),
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
                mint: whitelist_nft,
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
                mint: whitelist_nft,
                payer,
                metadata: nft_metadata,
                token_program,
                system_program,
                rent: Some(rent),
            },
            CreateMasterEditionV3InstructionArgs {
                max_supply: None,
            },
        );
        master_edition_account.invoke_signed(mint_auth_signer)?;

        self.sol_strategy.next_whitelist_id += 1;

        Ok(())
    }
}

#[error_code]
enum Error {
    #[msg("Caller is not the required governance authority defined in the SolStrategy.")]
    UnauthorizedGovernanceAuthority,
}
