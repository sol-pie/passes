use anchor_lang::prelude::*;

use instructions::*;

pub mod error;
pub mod instructions;
pub mod state;

mod common;
mod math;

#[cfg(test)]
mod tests;

#[cfg(feature = "devnet")]
pub mod constants {
    use solana_program::{pubkey, pubkey::Pubkey};
    pub const USDC_MINT_PUBKEY: Pubkey = pubkey!("WaoKNLQVDyBx388CfjaVeyNbs3MT2mPgAhoCfXyUvg8");
}

pub const USDC_DECIMALS: u8 = 6;
pub const ONE_USDC: u64 = 1_000_000;
pub const SOL_DECIMALS: u8 = 9;
pub const ONE_SOL: u64 = 1_000_000_000;

declare_id!("8j5vzygvZzkmFAQ186yPbr4vgVGFtSvmFyzE7KVXmB8Q");

#[program]
pub mod passes {
    use super::*;

    pub fn init(ctx: Context<Init>, protocol_fee_bps: u64, owner_fee_bps: u64) -> Result<()> {
        instructions::init(ctx, protocol_fee_bps, owner_fee_bps)
    }

    pub fn set_protocol_fee_bps(ctx: Context<SetFeePercent>, fee_bps: u64) -> Result<()> {
        instructions::set_protocol_fee_bps(ctx, fee_bps)
    }

    pub fn set_owner_fee_bps(ctx: Context<SetFeePercent>, fee_bps: u64) -> Result<()> {
        instructions::set_owner_fee_bps(ctx, fee_bps)
    }

    pub fn set_protocol_fee_dst(ctx: Context<SetProtocolFeeDst>) -> Result<()> {
        instructions::set_protocol_fee_dst(ctx)
    }

    pub fn issue_passes(ctx: Context<IssuePasses>, amount: u64) -> Result<()> {
        instructions::issue_passes(ctx, amount)
    }

    pub fn get_price(ctx: Context<GetPrice>, supply: u64, amount: u64) -> Result<u64> {
        instructions::get_price(ctx, supply, amount)
    }

    pub fn get_price_sol(ctx: Context<GetPrice>, supply: u64, amount: u64) -> Result<u64> {
        instructions::get_price_sol(ctx, supply, amount)
    }

    pub fn buy_passes(ctx: Context<BuyPasses>, amount: u64) -> Result<()> {
        instructions::buy_passes(ctx, amount)
    }

    pub fn buy_passes_sol(ctx: Context<BuyPassesSol>, amount: u64) -> Result<()> {
        instructions::buy_passes_sol(ctx, amount)
    }

    pub fn sell_passes(ctx: Context<SellPasses>, amount: u64) -> Result<()> {
        instructions::sell_passes(ctx, amount)
    }

    pub fn sell_passes_sol(ctx: Context<SellPassesSol>, amount: u64) -> Result<()> {
        instructions::sell_passes_sol(ctx, amount)
    }
}
