#![allow(unused_variables)]
#![allow(unused_imports)]

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{
    common::{calc_fee, calc_price, transfer_tokens_from_user},
    error::PassesError,
    state, ONE_USDC,
};

// Purchase passes from a specified passes owner by sending a certain amount of token as payment

#[derive(Accounts)]
pub struct BuyPasses<'info> {
    // signer
    #[account(mut)]
    pub buyer: Signer<'info>,

    // derived PDAs
    #[account{
        mut,
        seeds = [b"supply", passes_owner.key.as_ref(),],
        bump = passes_supply.bump
    }]
    passes_supply: Box<Account<'info, state::PassesSupply>>,

    // TODO it is security init_if_needed?
    #[account{
        init_if_needed,
        payer = buyer,
        space = state::PassesBalance::LEN,
        seeds = [b"balance", passes_owner.key.as_ref(), buyer.key.as_ref()],
        bump,
    }]
    passes_balance: Box<Account<'info, state::PassesBalance>>,

    #[account(
        seeds = [state::Config::SEED],
        bump = config.bump
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
    pub passes_owner: AccountInfo<'info>, // buy passes for the specified passes owner

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
        constraint = buyer_wallet.owner == buyer.key(),
        constraint = buyer_wallet.mint == payment_mint.key()
    )]
    buyer_wallet: Box<Account<'info, TokenAccount>>, // buyer's ATA wallet

    // programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

// Buy passes

pub fn buy_passes(ctx: Context<BuyPasses>, amount: u64) -> Result<()> {
    let supply = ctx.accounts.passes_supply.amount;
    let owner = ctx.accounts.passes_owner.key();
    let buyer = ctx.accounts.buyer.key();
    let config = &ctx.accounts.config;
    let passes_balance = &mut ctx.accounts.passes_balance;
    let passes_supply = &mut ctx.accounts.passes_supply;

    require!(supply > 0, PassesError::ZeroSupply);

    let price = calc_price(supply, amount);
    require!(price > 0, PassesError::ZeroPrice);

    // calc fees
    let protocol_fees = calc_fee(config.protocol_fee_bps, price)?;
    let owner_fees = calc_fee(config.owner_fee_bps, price)?;

    // send buyer's token to escrow wallet
    let from = ctx.accounts.buyer_wallet.to_account_info();
    let to = ctx.accounts.escrow_wallet.to_account_info();
    let authority = ctx.accounts.buyer.to_account_info();
    let token_program = ctx.accounts.token_program.to_account_info();
    // msg!("Buyer wallet: {:#?}", ctx.accounts.buyer_wallet);
    transfer_tokens_from_user(
        from.clone(),
        to,
        authority.clone(),
        token_program.clone(),
        price,
    )?;
    msg!("Send buyer payment to escrow wallet: {}", price);

    // send protocol fees
    let to = ctx.accounts.protocol_fee_wallet.to_account_info();
    transfer_tokens_from_user(
        from.clone(),
        to,
        authority.clone(),
        token_program.clone(),
        protocol_fees,
    )?;
    msg!("Send protocol fees: {}", protocol_fees);

    // send owner fees
    let to = ctx.accounts.owner_fee_wallet.to_account_info();
    transfer_tokens_from_user(from, to, authority, token_program, owner_fees)?;
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
