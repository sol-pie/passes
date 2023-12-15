use anchor_lang::prelude::*;
use anchor_lang::{
    accounts::{account::Account, program::Program, signer::Signer},
    system_program::System,
    Accounts,
};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::error::PassesError;
use crate::state;

#[derive(Accounts)]
pub struct IssuePasses<'info> {
    // signer
    #[account(mut)]
    pub owner: Signer<'info>,

    // derived PDAs
    #[account{
        init,
        payer = owner,
        space = state::PassesSupply::LEN,
        seeds = [b"supply", owner.key.as_ref(),],
        bump,
    }]
    passes_supply: Box<Account<'info, state::PassesSupply>>,

    #[account{
        init,
        payer = owner,
        space = state::PassesBalance::LEN,
        seeds = [b"balance", owner.key.as_ref(), owner.key.as_ref()],
        bump,
    }]
    passes_balance: Box<Account<'info, state::PassesBalance>>,

    #[account(
        seeds = [state::Config::SEED],
        bump = config.bump
    )]
    pub config: Box<Account<'info, state::Config>>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = payment_mint,
        associated_token::authority = owner
    )]
    pub owner_fee_wallet: Box<Account<'info, TokenAccount>>, // owner's ATA to get fees

    // accounts
    #[account(
        constraint = payment_mint.key() == config.payment_mint,
    )]
    pub payment_mint: Box<Account<'info, Mint>>, // e.g. USDC mint account

    // programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn issue_passes(ctx: Context<IssuePasses>, amount: u64) -> Result<()> {
    let supply = ctx.accounts.passes_supply.amount;
    let owner = ctx.accounts.owner.key();

    require!(supply == 0, PassesError::PassesAlreadyIssued);
    require!(amount > 0, PassesError::ZeroAmount);

    let passes_balance = &mut ctx.accounts.passes_balance;
    let passes_supply = &mut ctx.accounts.passes_supply;
    passes_balance.amount = passes_balance
        .amount
        .checked_add(amount)
        .ok_or(PassesError::MathOverflow)?;
    passes_supply.amount = passes_supply
        .amount
        .checked_add(amount)
        .ok_or(PassesError::MathOverflow)?;

    passes_balance.bump = ctx.bumps.passes_balance;
    passes_supply.bump = ctx.bumps.passes_supply;

    msg!("Issue passes: owner {}, amount {}", owner, amount);

    Ok(())
}
