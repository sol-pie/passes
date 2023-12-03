#![allow(unused_variables)]
#![allow(unused_imports)]

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{
    error::PassesError,
    state,
    utils::{calc_price, transfer_tokens_from_user},
    ONE_USDC,
};

// Purchase passes from a specified passes owner by sending a certain amount of token as payment

#[derive(Accounts)]
pub struct BuyPasses<'info> {
    // signer
    #[account(mut)]
    pub buyer: Signer<'info>,

    // derived PDAs
    #[account{
        init_if_needed,
        payer = buyer,
        space = state::PassesSupply::LEN,
        seeds = [b"supply", passes_owner.key.as_ref(),],
        bump,
    }]
    passes_supply: Box<Account<'info, state::PassesSupply>>,
    #[account{
        init_if_needed,
        payer = buyer,
        space = state::PassesBalance::LEN,
        seeds = [b"balance", passes_owner.key.as_ref(), buyer.key.as_ref()],
        bump,
    }]
    passes_balance: Box<Account<'info, state::PassesBalance>>,
    #[account(
        seeds = [b"state"],
        bump
    )]
    pub state: Box<Account<'info, state::Passes>>,
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = payment_mint,
        associated_token::authority = passes_owner
    )]
    pub owner_fee_token: Box<Account<'info, TokenAccount>>, // owner's ATA to get fees
    #[account(
        mut,
        seeds = [b"escrow", payment_mint.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = state
    )]
    pub escrow_wallet: Box<Account<'info, TokenAccount>>, // escrow wallet (associated token account) to store buyer payments

    // accounts
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub passes_owner: AccountInfo<'info>, // buy passes for the specified passes owner
    pub payment_mint: Box<Account<'info, Mint>>, // e.g. USDC mint account
    #[account(
        mut,
        constraint=protocol_fee_token.owner == state.admin.key(),
        constraint=protocol_fee_token.mint == payment_mint.key()
    )]
    pub protocol_fee_token: Box<Account<'info, TokenAccount>>, // protocol's ATA to get fees
    #[account(
        mut,
        constraint=buyer_wallet.owner == buyer.key(),
        constraint=buyer_wallet.mint == payment_mint.key()
    )]
    buyer_wallet: Box<Account<'info, TokenAccount>>, // buyer's ATA wallet that has already approved ?

    // programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

// Buy passes

pub fn buy_passes(ctx: Context<BuyPasses>, amount: u64) -> Result<()> {
    let supply = ctx.accounts.passes_supply.supply;
    let owner = ctx.accounts.passes_owner.key();
    let buyer = ctx.accounts.buyer.key();
    let state = &ctx.accounts.state;
    let passes_balance = &mut ctx.accounts.passes_balance;
    let passes_supply = &mut ctx.accounts.passes_supply;

    require!(supply > 0 || owner == buyer, PassesError::ZeroSupply);

    let price = calc_price(supply, amount);

    let protocol_fees = price * state.protocol_fee_pct / ONE_USDC;
    let owner_fees = price * state.owner_fee_pct / ONE_USDC;

    if price > 0 {
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
        let to = ctx.accounts.protocol_fee_token.to_account_info();
        transfer_tokens_from_user(
            from.clone(),
            to,
            authority.clone(),
            token_program.clone(),
            protocol_fees,
        )?;
        msg!("Send protocol fees: {}", protocol_fees);

        // send owner fees
        let to = ctx.accounts.owner_fee_token.to_account_info();
        transfer_tokens_from_user(from, to, authority, token_program, owner_fees)?;
        msg!("Send owner fees: {}", owner_fees);
    }

    // TODO replace to checked_add
    passes_balance.balance += amount;
    passes_supply.supply += amount;
    msg!(
        "Buy passes: owner {}, buyer {}, amount {}, price {}",
        owner,
        buyer,
        amount,
        price
    );

    Ok(())
}
