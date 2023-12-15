use anchor_lang::prelude::*;

use crate::state;

// Set protocol and owner fee percent

#[derive(Accounts)]
pub struct SetFeePercent<'info> {
    // signer
    #[account(
        mut,
        constraint = admin.key() == config.admin
    )]
    pub admin: Signer<'info>,

    // derived PDAs
    #[account(
        mut,
        seeds = [state::Config::SEED],
        bump = config.bump,
        // realloc = state::Config::LEN,
        // realloc::payer = admin,
        // realloc::zero = true,
        has_one = admin
    )]
    pub config: Account<'info, state::Config>,

    // programs
    pub system_program: Program<'info, System>,
}

pub fn set_protocol_fee_bps(ctx: Context<SetFeePercent>, fee_bps: u64) -> Result<()> {
    ctx.accounts.config.protocol_fee_bps = fee_bps;

    Ok(())
}

pub fn set_owner_fee_bps(ctx: Context<SetFeePercent>, fee_bps: u64) -> Result<()> {
    ctx.accounts.config.owner_fee_bps = fee_bps;

    Ok(())
}
