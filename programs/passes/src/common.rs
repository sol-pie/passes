use std::fmt::Display;

use anchor_lang::prelude::*;
use anchor_spl::token::Transfer;

use crate::{error::PassesError, ONE_SOL, ONE_USDC};

pub fn calc_price(supply: u64, amount: u64) -> u64 {
    let sum1 = if supply == 0 {
        0
    } else {
        (supply - 1) * supply * (2 * (supply - 1) + 1) / 6
    };

    let sum2 = if supply == 0 && amount == 1 {
        0
    } else {
        (supply - 1 + amount) * (supply + amount) * (2 * (supply - 1 + amount) + 1) / 6
    };

    let summation = sum2 - sum1;
    let price = (summation * ONE_USDC) / 160;

    if cfg!(feature = "local-testing") {
        msg!(
            "Calc: sum1 {}, sum2 {}, summation {}, price {}, amount {}, supply {}",
            sum1,
            sum2,
            summation,
            price,
            amount,
            supply
        );
    } else {
        msg!(
            "Calc: price {}, amount {}, supply {}",
            price,
            amount,
            supply
        );
    }
    price
}

pub fn calc_price_sol(supply: u64, amount: u64) -> u64 {
    let sum1 = if supply == 0 {
        0
    } else {
        (supply - 1) * supply * (2 * (supply - 1) + 1) / 6
    };

    let sum2 = if supply == 0 && amount == 1 {
        0
    } else {
        (supply - 1 + amount) * (supply + amount) * (2 * (supply - 1 + amount) + 1) / 6
    };

    let summation = sum2 - sum1;
    let price = (summation * ONE_SOL) / 1600;

    if cfg!(feature = "local-testing") {
        msg!(
            "Calc: sum1 {}, sum2 {}, summation {}, price {}, amount {}, supply {}",
            sum1,
            sum2,
            summation,
            price,
            amount,
            supply
        );
    } else {
        msg!(
            "Calc: price {}, amount {}, supply {}",
            price,
            amount,
            supply
        );
    }
    price
}

pub fn calc_fees(
    price: u64,
    protocol_fee_pct: u64,
    owner_fee_pct: u64,
    divider: u64,
) -> Result<(u64, u64)> {
    let protocol_fees = price
        .checked_mul(protocol_fee_pct)
        .ok_or(PassesError::MathOverflow)?
        .checked_div(divider)
        .ok_or(PassesError::MathOverflow)?;
    let owner_fees = price
        .checked_mul(owner_fee_pct)
        .ok_or(PassesError::MathOverflow)?
        .checked_div(divider)
        .ok_or(PassesError::MathOverflow)?;

    Ok((protocol_fees, owner_fees))
}

pub fn scale(amount: u64, decimals: u8) -> u64 {
    checked_mul(amount, 10u64.pow(decimals as u32)).unwrap()
}

pub fn checked_mul<T>(arg1: T, arg2: T) -> Result<T>
where
    T: num_traits::PrimInt + Display,
{
    arg1.checked_mul(&arg2).map(Ok).unwrap_or_else(|| {
        msg!("Error: Overflow in {} * {}", arg1, arg2);
        err!(PassesError::MathOverflow)
    })
}

pub fn transfer_tokens<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
    authority_seeds: &[&[&[u8]]],
) -> Result<()> {
    let ctx = CpiContext::new_with_signer(
        token_program,
        Transfer {
            from,
            to,
            authority,
        },
        authority_seeds,
    );

    anchor_spl::token::transfer(ctx, amount)
}

pub fn transfer_tokens_from_user<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let ctx = CpiContext::new(
        token_program,
        Transfer {
            from,
            to,
            authority,
        },
    );
    anchor_spl::token::transfer(ctx, amount)
}
