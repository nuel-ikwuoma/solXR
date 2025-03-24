use {
    crate::state::sol_strategy::SolStrategy,
    crate::{SOLXR_DECIMAL},
    anchor_lang::prelude::*,
    anchor_spl::{
        metadata::{
            create_metadata_accounts_v3, mpl_token_metadata::types::DataV2,
            CreateMetadataAccountsV3, Metadata,
        },
        token::{Mint, Token},
    },
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Validate address by deriving pda
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata: UncheckedAccount<'info>,

    // Instantiate strategy
    #[account(
        init,
        payer = payer,
        space = 8 + SolStrategy::INIT_SPACE,
        seeds = [SolStrategy::SEED_PREFIX],
        bump
    )]
    pub sol_strategy: Account<'info, SolStrategy>,

    // Create new mint account
    #[account(
        init,
        payer = payer,
        mint::decimals = SOLXR_DECIMAL,
        mint::authority = sol_strategy.key(),
        mint::freeze_authority = sol_strategy.key(),
        seeds = [b"mint"],
        bump

    )]
    pub mint: Account<'info, Mint>,

    pub token_metadata_program: Program<'info, Metadata>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn initialize_handler(ctx: Context<Initialize>, initial_pool_cap: u64,individual_address_cap: u64) -> Result<()> {
    // Initialize Sol Strategy
    ctx.accounts.sol_strategy.set_inner(SolStrategy {
        initial_pool_cap,
        individual_address_cap,
    });

    // Get the bump for the mint authority PDA
    let mint_auth_bump = ctx.bumps.sol_strategy;
    let mint_auth_seeds: &[&[u8]] = &[SolStrategy::SEED_PREFIX, &[mint_auth_bump]];
    let mint_auth_signer: &[&[&[u8]]] = &[&mint_auth_seeds[..]];

    // Cross Program Invocation (CPI)
    // Invoking the create_metadata_account_v3 instruction on the token metadata program
    create_metadata_accounts_v3(
        CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                mint: ctx.accounts.mint.to_account_info(),
                metadata: ctx.accounts.metadata.to_account_info(),
                payer: ctx.accounts.payer.to_account_info(),
                mint_authority: ctx.accounts.sol_strategy.to_account_info(),
                update_authority: ctx.accounts.sol_strategy.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
        ).with_signer(mint_auth_signer),
        DataV2 {
            name: "Solana Strategy Token".parse().unwrap(),
            symbol: "SOLXR".parse().unwrap(),
            uri: "https://bafybeid3epo2cqokmuli24hzsejyopvqp4lvi4s5fonkprrvteetlee7cu.ipfs.w3s.link/ipfs/bafybeid3epo2cqokmuli24hzsejyopvqp4lvi4s5fonkprrvteetlee7cu/metadata.json".parse().unwrap(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        false, // Is mutable
        true,  // Update authority is signer
        None,  // Collection details
    )?;

    Ok(())
}
