//! Error types

use anchor_lang::prelude::*;

#[error_code]
pub enum PassesError {
    #[msg("Only the passes' owner can buy the first pass")]
    ZeroSupply,
    #[msg("Cannot sell the last pass")]
    LastPass,
    #[msg("Insufficient passes")]
    InsufficientPasses,
    #[msg("Overflow in arithmetic operation")]
    MathOverflow,
    #[msg("The passes are already issued")]
    PassesAlreadyIssued,
    #[msg("Amount cannot be zero")]
    ZeroAmount,
    #[msg("Price cannot be zero")]
    ZeroPrice,
}
