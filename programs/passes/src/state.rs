use anchor_lang::prelude::*;

const DISCRIMINATOR_LENGTH: usize = 8;

#[account]
#[derive(Default, Debug)]
pub struct Passes {
    /// Contract admin
    pub admin: Pubkey,
    /// The mint account for payments
    pub payment_mint: Pubkey,
    /// The escrow wallet (associated token account) to store buyer payments
    pub escrow_wallet: Pubkey,
    /// The percentage of protocol fees
    pub protocol_fee_pct: u64,
    /// The percentage of owner fees
    pub owner_fee_pct: u64,
    // The destination address (associated token account) for receiving protocol fees
    pub protocol_fee_token: Pubkey,
}

impl Passes {
    pub const LEN: usize = DISCRIMINATOR_LENGTH + std::mem::size_of::<Passes>();
}

#[account]
#[derive(Default, Debug)]
pub struct PassesSupply {
    // The address of the passes owner
    pub owner: Pubkey,
    // The supply associated with the  passes owner
    pub supply: u64,
}

impl PassesSupply {
    pub const LEN: usize = DISCRIMINATOR_LENGTH + std::mem::size_of::<PassesSupply>();
}

#[account]
#[derive(Default, Debug)]
pub struct PassesBalance {
    // The address of the passes owner
    pub owner: Pubkey,
    // The address of the passes holder
    pub holder: Pubkey,
    // The passes balances for respective holder and owner
    pub balance: u64,
}

impl PassesBalance {
    pub const LEN: usize = DISCRIMINATOR_LENGTH + std::mem::size_of::<PassesBalance>();
}
