use std::str::FromStr;

use anchor_lang::{InstructionData, ToAccountMetas};
use passes::{accounts, instruction};
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Signer};

const URL: &str = "https://api.devnet.solana.com";
const PROGRAM_ID: &str = "8j5vzygvZzkmFAQ186yPbr4vgVGFtSvmFyzE7KVXmB8Q";
// const MINT_KEY: &str = "41BJfZjr9kKLX9MtQAxqhaPot9pEntALQtPdxtuEEtxY";

const KEY_PATH: &str = "../dev3-keypair.json";

fn main() {
    let rpc_client = RpcClient::new(URL);

    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

    let admin = read_keypair_file(KEY_PATH).expect("Failed to read keypair from JSON");

    // let mint = Pubkey::from_str(MINT_KEY).unwrap();

    let args = instruction::GetPrice {
        supply: 1,
        amount: 10,
    };
    let accounts = accounts::GetPrice {
        invoker: admin.pubkey(),
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
