use crate::constants::SOLXR_DECIMAL;
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

#[derive(Accounts)]
pub struct InitializeNFT<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + SolStrategy::INIT_SPACE,
        seeds = [SolStrategy::SEED_PREFIX],
        bump
    )]
    pub sol_strategy: Account<'info, SolStrategy>,

    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = sol_strategy.key(),
        mint::freeze_authority = sol_strategy.key(),
        seeds = [b"nft"],
        bump
    )]
    pub nft: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), nft.key().as_ref()],
        bump,
        seeds::program = metadata_program.key(),
    )]
    /// CHECK: Validated by PDA derivation
    pub nft_metadata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), nft.key().as_ref(), b"edition"],
        bump,
        seeds::program = metadata_program.key(),
    )]
    /// CHECK: Initialized by Metaplex
    pub master_edition: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = nft,
        associated_token::authority = payer,
    )]
    pub nft_token_account: Account<'info, TokenAccount>,

    pub metadata_program: Program<'info, Metadata>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> InitializeNFT<'info> {
    pub fn handler(
        &mut self,
        bumps: &InitializeNFTBumps,
        bond_price: u64,
    ) -> Result<()> {
        // todo: make sure it is a set address that can execute this instruction

        self.sol_strategy.bond_price = bond_price;

        let nft_metadata = &self.nft_metadata.to_account_info();
        let master_edition = &self.master_edition.to_account_info();
        let nft_mint = &self.nft.to_account_info();
        let authority = &self.sol_strategy.to_account_info();
        let payer = &self.payer.to_account_info();
        let system_program = &self.system_program.to_account_info();
        let token_program = &self.token_program.to_account_info();
        let metadata_program = &self.metadata_program.to_account_info();
        let rent = &self.rent.to_account_info();

        let mint_auth_bump = bumps.sol_strategy;
        let mint_auth_seeds: &[&[u8]] = &[SolStrategy::SEED_PREFIX, &[mint_auth_bump]];
        let mint_auth_signer: &[&[&[u8]]] = &[&mint_auth_seeds[..]];

        // Mint 1 NFT token
        let mint_cpi = CpiContext::new_with_signer(
            token_program.clone(),
            MintTo {
                mint: nft_mint.clone(),
                to: self.nft_token_account.to_account_info(),
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
                mint: nft_mint,
                mint_authority: authority,
                payer,
                update_authority: (authority, true),
                system_program,
                rent: Some(rent),
            },
            CreateMetadataAccountV3InstructionArgs {
                data: DataV2 {
                    name: "Solana Strategy Bond".to_owned(),
                    symbol: "BONDXR".to_owned(),
                    uri: "https://bafybeiauoz3l4ssofopdg36a4teo5at6paavgjzvyhcyr5e4bvk5fwqlpy.ipfs.w3s.link/metadata.json".to_owned(),
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
                mint: nft_mint,
                payer,
                metadata: nft_metadata,
                token_program,
                system_program,
                rent: Some(rent),
            },
            CreateMasterEditionV3InstructionArgs { max_supply: None },
        );
        master_edition_account.invoke_signed(mint_auth_signer)?;

        Ok(())
    }
}
