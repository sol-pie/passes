use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::state;

// Initialize contract setting authority (admin)

#[derive(Accounts)]
pub struct Init<'info> {
    // signer
    #[account(mut)]
    pub admin: Signer<'info>,

    // derived PDAs
    #[account(
        init,
        payer = admin,
        space = state::Config::LEN,
        seeds = [state::Config::SEED],
        bump
    )]
    pub config: Account<'info, state::Config>,

    #[account(
        init,
        payer = admin,
        seeds = [b"escrow", payment_mint.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = config
    )]
    pub escrow_token_wallet: Account<'info, TokenAccount>, // escrow wallet (associated token account) to store buyer payments

    #[account(
        init,
        payer = admin,
        space = state::EscrowSOL::LEN,
        seeds = [state::EscrowSOL::SEED],
        bump
    )]
    pub escrow_sol_wallet: Account<'info, state::EscrowSOL>, // escrow wallet for SOL payment

    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = payment_mint,
        associated_token::authority = admin
    )]
    pub protocol_fee_wallet: Account<'info, TokenAccount>, // protocol's ATA to get fees

    // #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    // pub program: Program<'info, Passes>,
    // #[account(constraint = program_data.upgrade_authority_address == Some(admin.key()))]
    // pub program_data: Account<'info, ProgramData>,

    // accounts
    pub payment_mint: Account<'info, Mint>, // e.g. USDC mint account

    // programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn init(ctx: Context<Init>, protocol_fee_bps: u64, owner_fee_bps: u64) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = *ctx.accounts.admin.key;
    config.payment_mint = ctx.accounts.payment_mint.key();
    config.escrow_token_wallet = ctx.accounts.escrow_token_wallet.key();
    config.escrow_sol_wallet = ctx.accounts.escrow_sol_wallet.key();
    config.protocol_fee_token_wallet = ctx.accounts.protocol_fee_wallet.key();
    config.protocol_fee_bps = protocol_fee_bps;
    config.owner_fee_bps = owner_fee_bps;
    config.bump = ctx.bumps.config;

    ctx.accounts.escrow_sol_wallet.bump = ctx.bumps.escrow_sol_wallet;

    msg!(
            "Init: program admin {}, config {}, payment mint {}, escrow token wallet {}, escrow sol wallet {}, protocol fee token wallet {}, protocol fee bps {}, owner fee bps {}",
            config.admin,
            config.key(),
            config.payment_mint,
            config.escrow_token_wallet,
            config.escrow_sol_wallet,
            config.protocol_fee_token_wallet,
            config.protocol_fee_bps,
            config.owner_fee_bps
        );

    Ok(())
}
