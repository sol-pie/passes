#![allow(unused_variables)]
#![allow(unused_imports)]

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount, Transfer},
};

use crate::{
    error::PassesError,
    state,
    utils::{calc_price, transfer_tokens, transfer_tokens_from_user},
    ONE_USDC,
};

// Enables passes holders to sell their passes back to the contract

#[derive(Accounts)]
pub struct SellPasses<'info> {
    // signer
    #[account(mut)]
    pub seller: Signer<'info>,

    // derived PDAs
    #[account{
        init_if_needed,
        payer = seller,
        space = state::PassesSupply::LEN,
        seeds = [b"supply", passes_owner.key.as_ref()],
        bump,
    }]
    passes_supply: Box<Account<'info, state::PassesSupply>>,
    #[account{
        init_if_needed,
        payer = seller,
        space = state::PassesBalance::LEN,
        seeds = [b"balance", passes_owner.key.as_ref(), seller.key.as_ref()],
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
        payer = seller,
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
    pub passes_owner: AccountInfo<'info>, // sell passes for the specified passes owner
    pub payment_mint: Box<Account<'info, Mint>>, // e.g. USDC mint account
    #[account(
        mut,
        constraint=protocol_fee_token.owner == state.admin.key(),
        constraint=protocol_fee_token.mint == payment_mint.key()
    )]
    pub protocol_fee_token: Box<Account<'info, TokenAccount>>, // protocol's ATA to get fees
    #[account(
        mut,
        constraint=seller_wallet.owner == seller.key(),
        constraint=seller_wallet.mint == payment_mint.key()
    )]
    seller_wallet: Box<Account<'info, TokenAccount>>, // seller's ATA wallet that has already approved ?

    // programs
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

// Sell passes

pub fn sell_passes(ctx: Context<SellPasses>, amount: u64) -> Result<()> {
    let supply = ctx.accounts.passes_supply.supply;
    let balance = ctx.accounts.passes_balance.balance;
    let owner = ctx.accounts.passes_owner.key();
    let seller = ctx.accounts.seller.key();
    let mint = ctx.accounts.payment_mint.key();
    let state = &ctx.accounts.state;
    let passes_balance = &mut ctx.accounts.passes_balance;
    let passes_supply = &mut ctx.accounts.passes_supply;

    require!(supply > amount, PassesError::LastPass);
    require!(balance >= amount, PassesError::InsufficientPasses);

    let price = calc_price(supply - amount, amount);

    let protocol_fees = price * state.protocol_fee_pct / ONE_USDC;
    let owner_fees = price * state.owner_fee_pct / ONE_USDC;

    if price > 0 {
        // send seller token for sold passes
        let from = ctx.accounts.escrow_wallet.to_account_info();
        let to = ctx.accounts.seller_wallet.to_account_info();
        let authority = ctx.accounts.state.to_account_info();
        let bump_vector = ctx.bumps.escrow_wallet.to_le_bytes();
        // let authority_seeds: &[&[&[u8]]] = &[&[b"escrow", bump_vector.as_ref()]];
        let inner1 = vec![b"escrow".as_ref(), mint.as_ref(), bump_vector.as_ref()];
        // let inner2 = vec![authority.key.as_ref()];
        // let inner3 = vec![to.key.as_ref()];
        let authority_seeds = [inner1.as_slice()];
        // let authority_seeds = [inner1.as_slice(), inner2.as_slice()];
        let token_program = ctx.accounts.token_program.to_account_info();
        // msg!("Buyer wallet: {:#?}", ctx.accounts.buyer_wallet);

        let transfer_instruction = Transfer {
            from,
            to,
            authority,
        };
        let cpi_ctx = CpiContext::new_with_signer(
            token_program.to_account_info(),
            transfer_instruction,
            authority_seeds.as_slice(),
        );
        anchor_spl::token::transfer(cpi_ctx, amount)?;

        // transfer_tokens(
        //     from.clone(),
        //     to,
        //     authority.clone(),
        //     token_program.clone(),
        //     price - protocol_fees - owner_fees,
        //     authority_seeds.as_slice(),
        // )?;
        msg!(
            "Send seller payment from escrow wallet: {}",
            price - protocol_fees - owner_fees
        );

        // send protocol fees
        let to = ctx.accounts.protocol_fee_token.to_account_info();
        // transfer_tokens_from_user(
        //     from.clone(),
        //     to,
        //     authority.clone(),
        //     token_program.clone(),
        //     protocol_fees,
        // )?;
        msg!("Send protocol fees: {}", protocol_fees);

        // send owner fees
        let to = ctx.accounts.owner_fee_token.to_account_info();
        // transfer_tokens_from_user(from, to, authority, token_program, owner_fees)?;
        msg!("Send owner fees: {}", owner_fees);
    }

    // TODO replace to checked_add
    passes_balance.balance -= amount;
    passes_supply.supply -= amount;
    msg!(
        "Sell passes: owner {}, seller {}, amount {}, price {}",
        owner,
        seller,
        amount,
        price
    );

    Ok(())
}
