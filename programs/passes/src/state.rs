use anchor_lang::prelude::*;

// TODO replace size_of - manually calculate the account size
const DISCRIMINATOR_LENGTH: usize = 8;

#[account]
#[derive(Default, Debug)]
pub struct Config {
    /// Contract admin
    pub admin: Pubkey,
    /// The mint account for payments
    pub payment_mint: Pubkey,
    /// The escrow wallet (associated token account) to store buyer payments
    pub escrow_token_wallet: Pubkey,
    /// The escrow wallet to store buyer payments in SOL
    pub escrow_sol_wallet: Pubkey,
    /// The protocol fees in bps
    pub protocol_fee_bps: u64,
    /// The percentage of owner fees
    pub owner_fee_bps: u64,
    /// The destination address (associated token account) for receiving protocol fees
    pub protocol_fee_token_wallet: Pubkey,

    pub bump: u8,
}

impl Config {
    pub const LEN: usize = DISCRIMINATOR_LENGTH + std::mem::size_of::<Config>();
    pub const SEED: &[u8] = b"config";
}

#[account]
#[derive(Default, Debug)]
pub struct PassesSupply {
    // The supply associated with the  passes owner
    pub amount: u64,

    pub bump: u8,
}

impl PassesSupply {
    pub const LEN: usize = DISCRIMINATOR_LENGTH + std::mem::size_of::<PassesSupply>();
    pub const SEED: &[u8] = b"supply";
}

#[account]
#[derive(Default, Debug)]
pub struct PassesBalance {
    // The passes balances for respective holder and owner
    pub amount: u64,

    pub bump: u8,
}

impl PassesBalance {
    pub const LEN: usize = DISCRIMINATOR_LENGTH + std::mem::size_of::<PassesBalance>();
    pub const SEED: &[u8] = b"balance";
}

#[account]
pub struct EscrowSOL {
    pub bump: u8,
}

impl EscrowSOL {
    pub const LEN: usize = DISCRIMINATOR_LENGTH + std::mem::size_of::<EscrowSOL>();
    pub const SEED: &[u8] = b"escrow";
}

// TODO pub for_future_use: [u8; 128],
