use std::fmt::Display;

use anchor_lang::prelude::*;
use solana_program::msg;

use crate::error::PassesError;

pub const BPS_DECIMALS: u8 = 4;
pub const BPS_POWER: u128 = 10u64.pow(BPS_DECIMALS as u32) as u128;

pub fn checked_as_u64<T>(arg: T) -> Result<u64>
where
    T: Display + num_traits::ToPrimitive + Clone,
{
    let option: Option<u64> = num_traits::NumCast::from(arg.clone());
    if let Some(res) = option {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} as u64", arg);
        err!(PassesError::MathOverflow)
    }
}

pub fn checked_ceil_div<T>(arg1: T, arg2: T) -> Result<T>
where
    T: num_traits::PrimInt + Display,
{
    if arg1 > T::zero() {
        if arg1 == arg2 && arg2 != T::zero() {
            return Ok(T::one());
        }
        if let Some(res) = (arg1 - T::one()).checked_div(&arg2) {
            Ok(res + T::one())
        } else {
            msg!("Error: Overflow in {} / {}", arg1, arg2);
            err!(PassesError::MathOverflow)
        }
    } else if let Some(res) = arg1.checked_div(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} / {}", arg1, arg2);
        err!(PassesError::MathOverflow)
    }
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

#[allow(dead_code)]
pub fn scale(amount: u64, decimals: u8) -> u64 {
    checked_mul(amount, 10u64.pow(decimals as u32)).unwrap()
}
