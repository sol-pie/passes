use anchor_lang::prelude::*;

use crate::utils::calc_price;

// Calc and return pass price based on supply and amount

#[derive(Accounts)]
pub struct GetPrice<'info> {
    // signer
    #[account(mut)]
    pub invoker: Signer<'info>,
}

pub fn get_price(_ctx: Context<GetPrice>, supply: u64, amount: u64) -> Result<u64> {
    Ok(calc_price(supply, amount))
}
