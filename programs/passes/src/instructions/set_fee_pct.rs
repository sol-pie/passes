use anchor_lang::prelude::*;

use crate::state;

// Set protocol and owner fee percent

#[derive(Accounts)]
pub struct SetFeePercent<'info> {
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

    // programs
    pub system_program: Program<'info, System>,
}

pub fn set_protocol_fee_pct(ctx: Context<SetFeePercent>, fee_pct: u64) -> Result<()> {
    ctx.accounts.state.protocol_fee_pct = fee_pct;

    Ok(())
}

pub fn set_owner_fee_pct(ctx: Context<SetFeePercent>, fee_pct: u64) -> Result<()> {
    ctx.accounts.state.owner_fee_pct = fee_pct;

    Ok(())
}
