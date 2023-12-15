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
        seeds = [state::Config::SEED],
        bump = config.bump,
        // realloc = state::Config::LEN,
        // realloc::payer = admin,
        // realloc::zero = true,
        constraint = admin.key() == config.admin
    )]
    pub config: Account<'info, state::Config>,

    #[account(
        associated_token::mint = config.payment_mint,
        associated_token::authority = admin
    )]
    protocol_fee_wallet: Account<'info, TokenAccount>, // token account to send fee

    // programs
    pub system_program: Program<'info, System>,
}

pub fn set_protocol_fee_dst(ctx: Context<SetProtocolFeeDst>) -> Result<()> {
    ctx.accounts.config.protocol_fee_token_wallet = ctx.accounts.protocol_fee_wallet.key();
    msg!(
        "Protocol fee token: {}",
        ctx.accounts.config.protocol_fee_token_wallet
    );

    Ok(())
}
