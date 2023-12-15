use anchor_lang::prelude::*;
use solana_program::system_instruction;

use crate::{
    common::{calc_fee, calc_price_sol},
    error::PassesError,
    state,
};

// Purchase passes from a specified passes owner by sending a certain amount of token as payment

#[derive(Accounts)]
pub struct BuyPassesSol<'info> {
    // signer
    #[account(mut)]
    pub buyer: Signer<'info>,

    // derived PDAs
    #[account{
        mut,
        seeds = [b"supply", passes_owner.key.as_ref(),],
        bump = passes_supply.bump,
    }]
    passes_supply: Box<Account<'info, state::PassesSupply>>,

    // TODO it is security init_if_needed?
    #[account{
        init_if_needed,
        payer = buyer,
        space = state::PassesBalance::LEN,
        seeds = [b"balance", passes_owner.key.as_ref(), buyer.key.as_ref()],
        bump
    }]
    passes_balance: Box<Account<'info, state::PassesBalance>>,

    #[account(
        seeds = [state::Config::SEED],
        bump = config.bump
    )]
    pub config: Box<Account<'info, state::Config>>,

    #[account(
        mut,
        seeds = [state::EscrowSOL::SEED],
        bump = escrow_wallet.bump
    )]
    pub escrow_wallet: Box<Account<'info, state::EscrowSOL>>,

    // accounts
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub passes_owner: AccountInfo<'info>, // buy passes for the specified passes owner

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
        constraint = protocol_fee_wallet.key() == config.admin
    )]
    pub protocol_fee_wallet: AccountInfo<'info>,

    // programs
    pub system_program: Program<'info, System>,
}

// Buy passes with SOL

pub fn buy_passes_sol(ctx: Context<BuyPassesSol>, amount: u64) -> Result<()> {
    let supply = ctx.accounts.passes_supply.amount;
    let owner = ctx.accounts.passes_owner.key();
    let buyer = ctx.accounts.buyer.key();
    let config = &ctx.accounts.config;
    let passes_balance = &mut ctx.accounts.passes_balance;
    let passes_supply = &mut ctx.accounts.passes_supply;

    require!(supply > 0, PassesError::ZeroSupply);

    let price = calc_price_sol(supply, amount);
    require!(price > 0, PassesError::ZeroPrice);

    // calc fees
    let protocol_fees = calc_fee(config.protocol_fee_bps, price)?;
    let owner_fees = calc_fee(config.owner_fee_bps, price)?;

    // send buyer's token to escrow wallet
    let from = ctx.accounts.buyer.to_account_info();
    let to = ctx.accounts.escrow_wallet.to_account_info();
    anchor_lang::solana_program::program::invoke(
        &system_instruction::transfer(from.key, to.key, price),
        &[
            from.clone(),
            to,
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;
    msg!("Send buyer payment to escrow wallet: {}", price);

    // send protocol fees
    let to = ctx.accounts.protocol_fee_wallet.to_account_info();
    anchor_lang::solana_program::program::invoke(
        &system_instruction::transfer(from.key, to.key, protocol_fees),
        &[
            from.clone(),
            to,
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;
    msg!("Send protocol fees: {}", protocol_fees);

    // send owner fees
    let to = ctx.accounts.passes_owner.clone();
    let cpi_context = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        anchor_lang::system_program::Transfer { from, to },
    );
    anchor_lang::system_program::transfer(cpi_context, owner_fees)?;
    msg!("Send owner fees: {}", owner_fees);

    passes_balance.amount = passes_balance
        .amount
        .checked_add(amount)
        .ok_or(PassesError::MathOverflow)?;
    passes_supply.amount = passes_supply
        .amount
        .checked_add(amount)
        .ok_or(PassesError::MathOverflow)?;

    passes_balance.bump = ctx.bumps.passes_balance;

    msg!(
        "Buy passes: owner {}, buyer {}, amount {}, price {}, protocol_fees {}, owner_fees {}, balance {}, supply {}",
        owner,
        buyer,
        amount,
        price,
        protocol_fees,
        owner_fees,
        passes_balance.amount,
        passes_supply.amount
    );

    Ok(())
}
