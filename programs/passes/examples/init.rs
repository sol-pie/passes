use std::str::FromStr;

use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::token::Mint;
use passes::{accounts, instruction};
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{read_keypair_file, Keypair, Signer},
    transaction::Transaction,
};

const _LOCALNET_URL: &str = "http://127.0.0.1:8899";
const DEVNET_URL: &str = "https://api.devnet.solana.com";
const KEY_PATH: &str = "../dev3-keypair.json";
const PROTOCOL_FEE_BPS: u64 = 100; // 100bps = 1%
const OWNER_FEE_BPS: u64 = 100; // 100bps = 1%
const USDC_DEV_MINT_ACC: &str = "Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr";
const _USDC_MINT_ACC: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

fn main() {
    let rpc_client = RpcClient::new(DEVNET_URL);

    let program_id = passes::id();

    let admin = read_keypair_file(KEY_PATH).expect("Failed to read keypair from JSON");
    // let mint = create_token(&rpc_client, &admin);

    let mint_key = Pubkey::from_str(USDC_DEV_MINT_ACC).unwrap();

    let config = Pubkey::find_program_address(&[b"config".as_slice()], &program_id).0;
    let escrow_token_wallet =
        Pubkey::find_program_address(&[b"escrow".as_slice(), mint_key.as_ref()], &program_id).0;
    let escrow_sol_wallet = Pubkey::find_program_address(&[b"escrow".as_slice()], &program_id).0;
    let protocol_fee_wallet =
        anchor_spl::associated_token::get_associated_token_address(&admin.pubkey(), &mint_key);

    let args = instruction::Init {
        protocol_fee_bps: PROTOCOL_FEE_BPS,
        owner_fee_bps: OWNER_FEE_BPS,
    };
    let accounts = accounts::Init {
        admin: admin.pubkey(),
        config,
        escrow_token_wallet,
        escrow_sol_wallet,
        protocol_fee_wallet,
        payment_mint: mint_key,
        system_program: system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    };

    // execute tx
    let last_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts: accounts.to_account_metas(None),
        data: args.data(),
    };
    let mut tx = solana_sdk::transaction::Transaction::new_with_payer(&[ix], Some(&admin.pubkey()));
    tx.sign(&[&admin], last_blockhash);

    let sig = rpc_client.send_and_confirm_transaction(&tx).unwrap();
    eprintln!("sig = {:#?}", sig);
}

fn _create_token(client: &RpcClient, owner: &Keypair) -> Keypair {
    let minimum_balance_for_rent_exemption = client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .unwrap();

    let new_mint_account = Keypair::new();

    let create_account_ix = solana_sdk::system_instruction::create_account(
        &owner.pubkey(),
        &new_mint_account.pubkey(),
        minimum_balance_for_rent_exemption,
        Mint::LEN as u64,
        &owner.pubkey(),
        // &spl_token::ID,
    );

    let _init_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &new_mint_account.pubkey(),
        &owner.pubkey(),
        None,
        9,
    )
    .unwrap();

    let latest_blockhash = client.get_latest_blockhash().unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[create_account_ix],
        Some(&owner.pubkey()),
        &[owner, &new_mint_account],
        latest_blockhash,
    );

    client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .unwrap();

    new_mint_account
}
