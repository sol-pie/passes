#![allow(unused_variables)]
#![allow(unused_imports)]

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount, Transfer},
};

use crate::{
    common::{calc_fees, calc_price, transfer_tokens, transfer_tokens_from_user},
    error::PassesError,
    state, ONE_USDC,
};

// Enables passes holders to sell their passes back to the contract

#[derive(Accounts)]
pub struct SellPasses<'info> {
    // signer
    #[account(mut)]
    pub seller: Signer<'info>,

    // derived PDAs
    #[account{
        mut,
        seeds = [b"supply", passes_owner.key.as_ref()],
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
        associated_token::mint = payment_mint,
        associated_token::authority = passes_owner
    )]
    pub owner_fee_wallet: Box<Account<'info, TokenAccount>>, // owner's ATA to get fees

    #[account(
        mut,
        seeds = [b"escrow", payment_mint.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = config
    )]
    pub escrow_wallet: Box<Account<'info, TokenAccount>>, // escrow wallet (associated token account) to store buyer payments

    // accounts
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub passes_owner: AccountInfo<'info>, // sell passes for the specified passes owner

    pub payment_mint: Box<Account<'info, Mint>>, // e.g. USDC mint account

    #[account(
        mut,
        constraint = protocol_fee_wallet.owner == config.admin.key(),
        constraint = protocol_fee_wallet.mint == payment_mint.key(),
        constraint = protocol_fee_wallet.key() == config.protocol_fee_token_wallet
    )]
    pub protocol_fee_wallet: Box<Account<'info, TokenAccount>>, // protocol's ATA to get fees

    #[account(
        mut,
        constraint = seller_wallet.owner == seller.key(),
        constraint = seller_wallet.mint == payment_mint.key()
    )]
    seller_wallet: Box<Account<'info, TokenAccount>>, // seller's ATA wallet that has already approved ?

    // programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

// Sell passes

pub fn sell_passes(ctx: Context<SellPasses>, amount: u64) -> Result<()> {
    let supply = ctx.accounts.passes_supply.amount;
    let balance = ctx.accounts.passes_balance.amount;
    let owner = ctx.accounts.passes_owner.key();
    let seller = ctx.accounts.seller.key();
    let mint = ctx.accounts.payment_mint.key();
    let config = &ctx.accounts.config;
    let config_key = &ctx.accounts.config.key();
    let passes_balance = &mut ctx.accounts.passes_balance;
    let passes_supply = &mut ctx.accounts.passes_supply;

    require!(supply > amount, PassesError::LastPass);
    require!(balance >= amount, PassesError::InsufficientPasses);

    let price = calc_price(supply - amount, amount);
    require!(price > 0, PassesError::ZeroPrice);

    let (protocol_fees, owner_fees) = calc_fees(
        price,
        config.protocol_fee_pct,
        config.owner_fee_pct,
        ONE_USDC,
    )?;

    // send seller token for sold passes
    let from = ctx.accounts.escrow_wallet.to_account_info();
    let to = ctx.accounts.seller_wallet.to_account_info();
    let authority = ctx.accounts.config.to_account_info();
    let bump_vector = ctx.bumps.config.to_le_bytes();
    let authority_seeds: &[&[&[u8]]] = &[&[b"config", bump_vector.as_ref()]];
    let token_program = ctx.accounts.token_program.to_account_info();
    let sent_amount = price
        .checked_sub(protocol_fees)
        .ok_or(PassesError::MathOverflow)?
        .checked_sub(owner_fees)
        .ok_or(PassesError::MathOverflow)?;
    transfer_tokens(
        from.clone(),
        to,
        authority.clone(),
        token_program.clone(),
        sent_amount,
        authority_seeds,
    )?;
    msg!(
        "Send pass price from escrow wallet to seller: {}",
        sent_amount
    );

    // send protocol fees
    let to = ctx.accounts.protocol_fee_wallet.to_account_info();
    transfer_tokens(
        from.clone(),
        to,
        authority.clone(),
        token_program.clone(),
        protocol_fees,
        authority_seeds,
    )?;
    msg!("Send protocol fees: {}", protocol_fees);

    // send owner fees
    let to = ctx.accounts.owner_fee_wallet.to_account_info();
    transfer_tokens(
        from,
        to,
        authority,
        token_program,
        owner_fees,
        authority_seeds,
    )?;
    msg!("Send owner fees: {}", owner_fees);

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
