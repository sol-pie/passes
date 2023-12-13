use anchor_lang::prelude::*;

use crate::{
    common::{calc_fees, calc_price_sol},
    error::PassesError,
    state, ONE_SOL,
};

// Enables passes holders to sell their passes back to the contract

#[derive(Accounts)]
pub struct SellPassesSol<'info> {
    // signer
    #[account(mut)]
    pub seller: Signer<'info>,

    // derived PDAs
    #[account{
        mut,
        seeds = [b"supply", passes_owner.key.as_ref(),],
        bump,
    }]
    passes_supply: Box<Account<'info, state::PassesSupply>>,

    #[account{
        mut,
        seeds = [b"balance", passes_owner.key.as_ref(), seller.key.as_ref()],
        bump,
    }]
    passes_balance: Box<Account<'info, state::PassesBalance>>,

    #[account(
        seeds = [state::Config::SEED],
        bump
    )]
    pub config: Box<Account<'info, state::Config>>,

    #[account(
        mut,
        seeds = [state::EscrowSOL::SEED],
        bump
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

// Sell passes

pub fn sell_passes_sol(ctx: Context<SellPassesSol>, amount: u64) -> Result<()> {
    let supply = ctx.accounts.passes_supply.amount;
    let balance = ctx.accounts.passes_balance.amount;
    let owner = ctx.accounts.passes_owner.key();
    let seller = ctx.accounts.seller.key();
    let config = &ctx.accounts.config;
    let passes_balance = &mut ctx.accounts.passes_balance;
    let passes_supply = &mut ctx.accounts.passes_supply;

    require!(supply > amount, PassesError::LastPass);
    require!(balance >= amount, PassesError::InsufficientPasses);

    let price = calc_price_sol(supply - amount, amount);

    let (protocol_fees, owner_fees) = calc_fees(
        price,
        config.protocol_fee_pct,
        config.owner_fee_pct,
        ONE_SOL,
    )?;
    require!(price > 0, PassesError::ZeroPrice);

    // msg!("config {:#?}", ctx.accounts.config.to_account_info());
    // msg!("escrow {:#?}", ctx.accounts.escrow_wallet.to_account_info());
    // msg!("passes_supply {:#?}", passes_supply.to_account_info());
    // msg!(
    //     "passes_owner {:#?}",
    //     ctx.accounts.passes_owner.to_account_info()
    // );

    // send SOL to seller for sold passes
    let sent_amount = price
        .checked_sub(protocol_fees)
        .ok_or(PassesError::MathOverflow)?
        .checked_sub(owner_fees)
        .ok_or(PassesError::MathOverflow)?;
    ctx.accounts.escrow_wallet.sub_lamports(sent_amount)?;
    ctx.accounts.seller.add_lamports(sent_amount)?;

    // send protocol fees
    ctx.accounts.escrow_wallet.sub_lamports(protocol_fees)?;
    ctx.accounts
        .protocol_fee_wallet
        .add_lamports(protocol_fees)?;

    // send owner fees
    ctx.accounts.escrow_wallet.sub_lamports(owner_fees)?;
    ctx.accounts.passes_owner.add_lamports(owner_fees)?;

    passes_balance.amount = passes_balance
        .amount
        .checked_sub(amount)
        .ok_or(PassesError::MathOverflow)?;
    passes_supply.amount = passes_supply
        .amount
        .checked_sub(amount)
        .ok_or(PassesError::MathOverflow)?;

    msg!(
        "Sell passes: owner {}, seller {}, amount {}, price {}, protocol_fees {}, owner_fees {}, balance {}, supply {}",
        owner,
        seller,
        amount,
        price,
        protocol_fees,
        owner_fees,
        passes_balance.amount,
        passes_supply.amount
        );

    Ok(())
}
