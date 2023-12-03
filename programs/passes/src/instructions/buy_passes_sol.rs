use anchor_lang::prelude::*;
use solana_program::system_instruction;

use crate::{error::PassesError, state, utils::calc_price_sol, ONE_SOL};

// Purchase passes from a specified passes owner by sending a certain amount of token as payment

#[derive(Accounts)]
pub struct BuyPassesSol<'info> {
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
        mut,
        seeds = [b"state"],
        bump
    )]
    pub state: Box<Account<'info, state::Passes>>,

    // accounts
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub passes_owner: AccountInfo<'info>, // buy passes for the specified passes owner
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub admin: AccountInfo<'info>, // buy passes for the specified passes owner

    // programs
    pub system_program: Program<'info, System>,
    // pub token_program: Program<'info, Token>,
    // pub associated_token_program: Program<'info, AssociatedToken>,
}

// Buy passes

pub fn buy_passes_sol(ctx: Context<BuyPassesSol>, amount: u64) -> Result<()> {
    let supply = ctx.accounts.passes_supply.supply;
    let owner = ctx.accounts.passes_owner.key();
    let buyer = ctx.accounts.buyer.key();
    let state = &ctx.accounts.state;
    let passes_balance = &mut ctx.accounts.passes_balance;
    let passes_supply = &mut ctx.accounts.passes_supply;

    require!(supply > 0 || owner == buyer, PassesError::ZeroSupply);

    let price = calc_price_sol(supply, amount);

    let protocol_fees = price * state.protocol_fee_pct / ONE_SOL;
    let owner_fees = price * state.owner_fee_pct / ONE_SOL;

    if price > 0 {
        // send buyer's token to escrow wallet
        let from = ctx.accounts.buyer.to_account_info();
        let to = ctx.accounts.admin.to_account_info();

        // Invoke the transfer instruction
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(from.key, to.key, amount),
            &[
                from.clone(),
                to.clone(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        msg!("Send buyer payment to escrow wallet: {}", price);

        // send protocol fees
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
        let to = ctx.accounts.passes_owner.key;
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(from.key, to, owner_fees),
            &[
                from,
                ctx.accounts.passes_owner.clone(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
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
