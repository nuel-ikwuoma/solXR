use {
    crate::{
        state::{
            bonds::{Bond, BondRecord},
            sol_strategy::SolStrategy,
        },
    },
    anchor_lang::prelude::*,
    anchor_lang::system_program,
    anchor_spl::{
        associated_token::AssociatedToken,
        metadata::{
            mpl_token_metadata::{
                accounts::MasterEdition,
                instructions::{
                    MintNewEditionFromMasterEditionViaTokenCpi,
                    MintNewEditionFromMasterEditionViaTokenCpiAccounts,
                    MintNewEditionFromMasterEditionViaTokenInstructionArgs,
                },
                types::{MintNewEditionFromMasterEditionViaTokenArgs},
            },
            Metadata,
        },
        token::{mint_to, Mint, MintTo, Token, TokenAccount},
    },
};

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct BuyBond<'info> {
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
        init_if_needed,
        payer = buyer,
        space = 8 + BondRecord::INIT_SPACE,
        seeds = [BondRecord::SEED_PREFIX, bond.key().as_ref(), buyer.key().as_ref()],
        bump
    )]
    pub bond_record: Account<'info, BondRecord>,

    #[account(
        init,
        payer = buyer,
        mint::decimals = 0,
        mint::authority = buyer,
        mint::freeze_authority = buyer,
        seeds = [bond.key().as_ref(), bond.next_edition_number.to_le_bytes().as_ref()],
        bump
    )]
    pub buyer_bond_nft: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = buyer_bond_nft,
        associated_token::authority = buyer,
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,
    /// CHECK: Buyer Bond Metadata account of the master edition NFT. Validated by derivation by MPL program.
    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), buyer_bond_nft.key().as_ref()],
        bump,
        seeds::program = metadata_program.key(),
    )]
    pub buyer_metadata: AccountInfo<'info>,
    /// CHECK: Buyer Bond Edition of the master edition NFT. Validated by derivation by MPL program.
    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), buyer_bond_nft.key().as_ref(), b"edition"],
        bump,
        seeds::program = metadata_program.key(),
    )]
    pub buyer_edition: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [bond.key().as_ref()],
        bump
    )]
    pub bond_nft: Account<'info, Mint>,
    #[account(
        associated_token::mint = bond_nft,
        associated_token::authority = sol_strategy,
    )]
    pub bond_token_account: Account<'info, TokenAccount>,
    /// CHECK: Bond Metadata account of the master edition NFT. Validated by derivation by MPL program.
    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), bond_nft.key().as_ref()],
        bump,
        seeds::program = metadata_program.key(),
    )]
    pub bond_metadata: UncheckedAccount<'info>,
    /// CHECK: Bond Edition of the master edition NFT. Validated by derivation by MPL program.
    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), bond_nft.key().as_ref(),b"edition"],
        bump,
        seeds::program = metadata_program.key(),
    )]
    pub bond_edition: UncheckedAccount<'info>,
    /// CHECK: PDA derived for the specific edition number, created by MPL CPI. Needs to be mutable and signer (payer).
    #[account(
        mut,
        seeds = [b"metadata", metadata_program.key().as_ref(), bond_nft.key().as_ref(), b"edition", bond.next_edition_marker.as_bytes()],
        bump,
        seeds::program = metadata_program.key(),
    )]
    pub edition_mark_pda: UncheckedAccount<'info>,

    pub metadata_program: Program<'info, Metadata>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> BuyBond<'info> {
    pub fn handler(&mut self, bumps: &BuyBondBumps, _id: u64) -> Result<()> {
        let sol_strategy = &mut self.sol_strategy;
        let bond = &mut self.bond;
        let buyer = &mut self.buyer;
        let clock = Clock::get()?;

        require!(
            clock.unix_timestamp as u64 >= bond.start_time,
            Error::MintingNotStarted
        );
        require!(
            clock.unix_timestamp as u64 <= bond.end_time,
            Error::MintingEnded
        );

        let bond_record = &mut self.bond_record;
        require!(
            bond_record.minted < bond.max_mint_per_wallet,
            Error::MaxMintPerWalletReached
        );

        system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: buyer.to_account_info(),
                    to: self.treasury.to_account_info(),
                },
            ),
            bond.price,
        )?;

        let current_supply: u64;
        let next_edition_number: u64;
        {
            let master_edition_account_info = self.bond_edition.to_account_info();
            let master_edition_data = master_edition_account_info.try_borrow_data()?;
            let edition_deserializer = &mut &master_edition_data[..];
            let master_edition_account = match MasterEdition::deserialize(edition_deserializer) {
                Ok(account) => account,
                Err(_) => return err!(Error::AccountNotMasterEdition),
            };

            current_supply = master_edition_account.supply;
            next_edition_number = current_supply
                .checked_add(1)
                .ok_or(Error::EditionOverflow)?;

            if let Some(max_supply) = master_edition_account.max_supply {
                require!(current_supply < max_supply, Error::MaxSupplyReached); // todo: test supply
            }
        }
        let sol_strategy_bump = bumps.sol_strategy;
        let sol_strategy_seeds: &[&[u8]] = &[SolStrategy::SEED_PREFIX, &[sol_strategy_bump]];
        let signer_seeds: &[&[&[u8]]] = &[&sol_strategy_seeds[..]];
        let rent = &self.rent.to_account_info();

        let mint_to_cpi_accounts = MintTo {
            mint: self.buyer_bond_nft.to_account_info(),
            to: self.buyer_token_account.to_account_info(),
            authority: buyer.to_account_info(),
        };
        let mint_to_cpi_ctx =
            CpiContext::new(self.token_program.to_account_info(), mint_to_cpi_accounts);
        mint_to(mint_to_cpi_ctx, 1)?;

        let cpi_accounts = MintNewEditionFromMasterEditionViaTokenCpiAccounts {
            new_metadata: &self.buyer_metadata.to_account_info(),
            new_edition: &self.buyer_edition.to_account_info(),
            master_edition: &self.bond_edition.to_account_info(),
            new_mint: &self.buyer_bond_nft.to_account_info(),
            edition_mark_pda: &self.edition_mark_pda.to_account_info(),
            new_mint_authority: &buyer.to_account_info(),
            payer: &buyer.to_account_info(),
            token_account_owner: &sol_strategy.to_account_info(),
            token_account: &self.bond_token_account.to_account_info(),
            new_metadata_update_authority: &buyer.to_account_info(),
            metadata: &self.bond_metadata.to_account_info(),
            token_program: &self.token_program.to_account_info(),
            system_program: &self.system_program.to_account_info(),
            rent: Some(rent),
        };

        let instruction_args = MintNewEditionFromMasterEditionViaTokenInstructionArgs {
            mint_new_edition_from_master_edition_via_token_args:
                MintNewEditionFromMasterEditionViaTokenArgs {
                    edition: next_edition_number,
                },
        };

        MintNewEditionFromMasterEditionViaTokenCpi::new(
            &self.metadata_program.to_account_info(),
            cpi_accounts,
            instruction_args,
        )
        .invoke_signed(signer_seeds)?;

        self.bond_record.set_inner(BondRecord {
            collection: bond.key(),
            user: buyer.key(),
            minted: 1,
        });
        bond.next_edition_number = next_edition_number + 1;
        bond.next_edition_marker = (next_edition_number + 1).checked_div(248).ok_or(Error::EditionOverflow)?.to_string();
        sol_strategy.sol_from_bond += bond.price;

        Ok(())
    }
}

#[error_code]
enum Error {
    #[msg("Minting has not yet started.")]
    MintingNotStarted,
    #[msg("Minting period has ended.")]
    MintingEnded,
    #[msg("Maximum mints per wallet reached.")]
    MaxMintPerWalletReached,
    #[msg("Provided account is not a valid Master Edition account.")]
    AccountNotMasterEdition,
    #[msg("Cannot mint more editions, supply reached.")]
    EditionOverflow,
    #[msg("Master edition max supply reached.")]
    MaxSupplyReached,
}
