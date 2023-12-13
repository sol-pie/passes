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

const URL: &str = "https://api.devnet.solana.com";
const PROGRAM_ID: &str = "8j5vzygvZzkmFAQ186yPbr4vgVGFtSvmFyzE7KVXmB8Q";
const KEY_PATH: &str = "../dev3-keypair.json";
const PROTOCOL_FEE_PCT: u64 = 500_000_000; // 0.5*10^9
const OWNER_FEE_PCT: u64 = 500_000_000; // 0.5*10^9

fn main() {
    let rpc_client = RpcClient::new(URL);

    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

    let admin = read_keypair_file(KEY_PATH).expect("Failed to read keypair from JSON");
    let mint = create_token(&rpc_client, &admin);

    let (config, _) = Pubkey::find_program_address(&[b"config".as_slice()], &program_id);
    let (escrow_token_wallet, _) =
        Pubkey::find_program_address(&[b"escrow".as_slice(), mint.pubkey().as_ref()], &program_id);
    let (escrow_sol_wallet, _) = Pubkey::find_program_address(&[b"escrow".as_slice()], &program_id);

    let protocol_fee_wallet =
        anchor_spl::associated_token::get_associated_token_address(&admin.pubkey(), &mint.pubkey());

    let args = instruction::Init {
        protocol_fee_pct: PROTOCOL_FEE_PCT,
        owner_fee_pct: OWNER_FEE_PCT,
    };
    let accounts = accounts::Init {
        admin: admin.pubkey(),
        config,
        escrow_token_wallet,
        escrow_sol_wallet,
        protocol_fee_wallet,
        payment_mint: mint.pubkey(),
        system_program: system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    }
    .to_account_metas(None);

    // execute tx
    let last_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let ix = solana_sdk::instruction::Instruction {
        program_id,
        accounts,
        data: args.data(),
    };
    let mut tx = solana_sdk::transaction::Transaction::new_with_payer(&[ix], Some(&admin.pubkey()));
    tx.sign(&[&admin], last_blockhash);

    let res = rpc_client.send_and_confirm_transaction(&tx);
    eprintln!("res = {:#?}", res);
}

fn create_token(client: &RpcClient, owner: &Keypair) -> Keypair {
    let minimum_balance_for_rent_exemption = client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .unwrap();

    let new_mint_account = Keypair::new();

    let create_account_ix = solana_sdk::system_instruction::create_account(
        &owner.pubkey(),
        &new_mint_account.pubkey(),
        minimum_balance_for_rent_exemption,
        Mint::LEN as u64,
        &spl_token::ID,
    );

    let init_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &new_mint_account.pubkey(),
        &owner.pubkey(),
        None,
        9,
    )
    .unwrap();

    let latest_blockhash = client.get_latest_blockhash().unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[create_account_ix, init_mint_ix],
        Some(&owner.pubkey()),
        &[owner, &new_mint_account],
        latest_blockhash,
    );

    client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .unwrap();

    new_mint_account
}
