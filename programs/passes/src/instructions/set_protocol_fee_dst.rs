use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::state;

// Set the destination address for receiving protocol fees

#[derive(Accounts)]
pub struct SetProtocolFeeDst<'info> {
    // signer
    #[account(mut)]
    pub admin: Signer<'info>,

    // derived PDAs
    #[account(
        mut,
        seeds = [b"state"],
        bump,
        realloc = state::Passes::LEN,
        realloc::payer = admin,
        realloc::zero = true,
        has_one = admin
    )]
    pub state: Account<'info, state::Passes>,
    #[account(
        associated_token::mint = state.payment_mint,
        associated_token::authority = admin
    )]
    protocol_fee_token: Account<'info, TokenAccount>, // token account to send fee

    // programs
    pub system_program: Program<'info, System>,
    // pub token_program: Program<'info, Token>,
    // pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn set_protocol_fee_dst(ctx: Context<SetProtocolFeeDst>) -> Result<()> {
    ctx.accounts.state.protocol_fee_token = ctx.accounts.protocol_fee_token.key();
    msg!(
        "Protocol fee token: {}",
        ctx.accounts.state.protocol_fee_token
    );

    Ok(())
}
