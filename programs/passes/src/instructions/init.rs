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
        space = state::Passes::LEN,
        seeds = [b"state"],
        bump
    )]
    pub state: Account<'info, state::Passes>,
    #[account(
        init,
        payer = admin,
        seeds = [b"escrow", payment_mint.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = state
    )]
    pub escrow_wallet: Account<'info, TokenAccount>, // escrow wallet (associated token account) to store buyer payments
    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = payment_mint,
        associated_token::authority = admin
    )]
    pub protocol_fee_token: Account<'info, TokenAccount>, // protocol's ATA to get fees

    // accounts
    pub payment_mint: Account<'info, Mint>, // e.g. USDC mint account

    // programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn init(ctx: Context<Init>, protocol_fee_pct: u64, owner_fee_pct: u64) -> Result<()> {
    let state = &mut ctx.accounts.state;

    state.admin = *ctx.accounts.admin.key;
    state.payment_mint = ctx.accounts.payment_mint.key();
    state.escrow_wallet = ctx.accounts.escrow_wallet.key();
    state.protocol_fee_token = ctx.accounts.protocol_fee_token.key();
    state.protocol_fee_pct = protocol_fee_pct;
    state.owner_fee_pct = owner_fee_pct;

    msg!(
        "Init: program admin {}, payment mint {}, escrow wallet {}, protocol fee token {}, protocol fee pct {}, owner fee {}",
        state.admin,
        state.payment_mint,
        state.escrow_wallet,
        state.protocol_fee_token,
        state.protocol_fee_pct,
        state.owner_fee_pct
    );

    Ok(())
}
