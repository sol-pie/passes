#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use std::collections::HashMap;

use anchor_lang::{
    prelude::{borsh::BorshDeserialize, *},
    system_program, InstructionData,
};
use assert_matches::*;
use bonfida_test_utils::ProgramTestContextExt;
use bonfida_test_utils::ProgramTestExt;
use maplit::hashmap;
use solana_program::program_pack::Pack;
use solana_program_test::{tokio, BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account,
    instruction::AccountMeta,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

use crate::{
    accounts::{self},
    instruction::{self},
    state::Passes,
    ONE_USDC, USDC_DECIMALS,
};

// TODO bps?
// const PROTOCOL_FEE_PCT: u64 = 500_000; // 0.5*10^6
// const OWNER_FEE_PCT: u64 = 500_000; // 0.5*10^6
const PROTOCOL_FEE_PCT: u64 = 500_000_000; // 0.5*10^9
const OWNER_FEE_PCT: u64 = 500_000_000; // 0.5*10^9
const TICKER: &str = "usdc";

#[derive(Debug)]
pub struct MintInfo {
    pub decimals: u8,
    pub pubkey: Pubkey,
}

#[tokio::test]
async fn test_init() {
    let (mut ctx, initializer, _, _, mints) = setup().await;

    init_passes(&mut ctx, &initializer, &mints[TICKER].pubkey).await;

    // check protocol fee percent
    let (state, _) = get_state_pda();
    let state_account = ctx.banks_client.get_account(state).await.unwrap().unwrap();
    let state = Passes::try_deserialize_unchecked(&mut state_account.data.as_slice()).unwrap();
    assert_eq!(state.admin, initializer.pubkey());
}

// #[ignore]
#[tokio::test]
async fn test_get_price() {
    let (mut ctx, initializer, _, _, _) = setup().await;

    // calc and get price
    let args = instruction::GetPrice {
        supply: 3,
        amount: 1,
    };
    let accounts = accounts::GetPrice {
        invoker: initializer.pubkey(),
    };
    let res = simulate_tx(
        &mut ctx,
        accounts.to_account_metas(None),
        &args,
        &initializer,
    )
    .await;
    assert_matches!(res, Ok(56250_u64));
}

#[tokio::test]
async fn test_set_fees_pct() {
    let (mut ctx, initializer, _, _, mints) = setup().await;

    init_passes(&mut ctx, &initializer, &mints[TICKER].pubkey).await;

    // set protocol fee percent
    let (state, _) = get_state_pda();
    let args = instruction::SetProtocolFeePct {
        fee_pct: PROTOCOL_FEE_PCT,
    };
    let accounts = accounts::SetFeePercent {
        admin: initializer.pubkey(),
        state,
        system_program: system_program::ID,
    };
    let res = execute_tx(
        &mut ctx,
        accounts.to_account_metas(None),
        &args,
        &initializer,
    )
    .await;
    assert_matches!(res, Ok(()));

    // set owner fee percent
    let args = instruction::SetOwnerFeePct {
        fee_pct: OWNER_FEE_PCT,
    };
    let accounts = accounts::SetFeePercent {
        admin: initializer.pubkey(),
        state,
        system_program: system_program::ID,
    };
    let res = execute_tx(
        &mut ctx,
        accounts.to_account_metas(None),
        &args,
        &initializer,
    )
    .await;
    assert_matches!(res, Ok(()));

    // check protocol fee percent
    let state_account = ctx.banks_client.get_account(state).await.unwrap().unwrap();
    let state = Passes::try_deserialize_unchecked(&mut state_account.data.as_slice()).unwrap();
    assert_eq!(state.protocol_fee_pct, PROTOCOL_FEE_PCT);
    assert_eq!(state.owner_fee_pct, OWNER_FEE_PCT);
    eprintln!("state = {:#?}", state);
}

#[tokio::test]
async fn test_set_fee_dst() {
    let (mut ctx, initializer, _, _, mints) = setup().await;

    init_passes(&mut ctx, &initializer, &mints[TICKER].pubkey).await;

    // set protocol fee dst
    let (state, _) = get_state_pda();
    let fee_token = anchor_spl::associated_token::get_associated_token_address(
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
    );
    let args = instruction::SetProtocolFeeDst {};
    let accounts = accounts::SetProtocolFeeDst {
        admin: initializer.pubkey(),
        state,
        protocol_fee_token: fee_token,
        system_program: system_program::ID,
    };

    let res = execute_tx(
        &mut ctx,
        accounts.to_account_metas(None),
        &args,
        &initializer,
    )
    .await;
    assert_matches!(res, Ok(()));

    // check protocol fee percent
    let state_account = ctx.banks_client.get_account(state).await.unwrap().unwrap();
    let state = Passes::try_deserialize_unchecked(&mut state_account.data.as_slice()).unwrap();
    eprintln!("state = {:#?}", state);
    assert_eq!(state.payment_mint, mints["usdc"].pubkey);
    assert_eq!(state.protocol_fee_token, fee_token);
}

// #[ignore]
#[tokio::test]
async fn test_buy_passes_by_owner() {
    let (mut ctx, initializer, buyer, owner, mints) = setup().await;

    init_passes(&mut ctx, &initializer, &mints[TICKER].pubkey).await;

    buy_passes(
        &mut ctx,
        &buyer,
        &buyer.pubkey(),
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
        1,
    )
    .await
}

#[tokio::test]
async fn test_buy_passes_by_buyer() {
    let (mut ctx, initializer, buyer, owner, mints) = setup().await;

    init_passes(&mut ctx, &initializer, &mints[TICKER].pubkey).await;

    // owner buy the first pass
    buy_passes(
        &mut ctx,
        &owner,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
        1,
    )
    .await;

    // buyer buy next passes
    buy_passes(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
        10,
    )
    .await
}

#[tokio::test]
async fn test_buy_passes_sol_by_owner() {
    let (mut ctx, initializer, buyer, owner, mints) = setup().await;

    init_passes(&mut ctx, &initializer, &mints[TICKER].pubkey).await;

    buy_passes_sol(
        &mut ctx,
        &buyer,
        &buyer.pubkey(),
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
        1,
    )
    .await
}

#[tokio::test]
async fn test_buy_passes_sol_by_buyer() {
    let (mut ctx, initializer, buyer, owner, mints) = setup().await;

    init_passes(&mut ctx, &initializer, &mints[TICKER].pubkey).await;

    // owner buy the first pass
    buy_passes_sol(
        &mut ctx,
        &owner,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
        1,
    )
    .await;

    // buyer buy next passes
    buy_passes_sol(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
        10,
    )
    .await
}

#[tokio::test]
async fn test_sell_passes() {
    let (mut ctx, initializer, buyer, owner, mints) = setup().await;

    init_passes(&mut ctx, &initializer, &mints[TICKER].pubkey).await;

    // owner buy the first pass
    buy_passes(
        &mut ctx,
        &owner,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
        1,
    )
    .await;

    // buyer buy next passes
    buy_passes(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
        10,
    )
    .await;

    // buyer sell part of their passes
    sell_passes(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mints[TICKER].pubkey,
        5,
    )
    .await;
}

async fn setup() -> (
    ProgramTestContext,
    Keypair,
    Keypair,
    Keypair,
    HashMap<String, MintInfo>,
) {
    let mut program_test = ProgramTest::new("passes", crate::id(), None);

    let initializer = Keypair::new();
    create_and_fund_account(&mut program_test, &initializer.pubkey());

    let buyer = Keypair::new();
    create_and_fund_account(&mut program_test, &buyer.pubkey());

    let owner = Keypair::new();
    create_and_fund_account(&mut program_test, &owner.pubkey());

    let mints = init_mints(&mut program_test, &initializer.pubkey());

    let mut ctx = program_test.start_with_context().await;

    init_and_fund_token_account(
        &mut ctx,
        &mints[TICKER].pubkey,
        &buyer.pubkey(),
        &initializer,
        5_u64 * ONE_USDC,
    )
    .await;

    // ctx.initialize_token_accounts(mints[TICKER].pubkey, &[buyer.pubkey()])
    //     .await
    //     .unwrap();
    ctx.initialize_token_accounts(mints[TICKER].pubkey, &[owner.pubkey()])
        .await
        .unwrap();

    (ctx, initializer, buyer, owner, mints)
}

fn get_state_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"state".as_slice()], &crate::id())
}

fn get_escrow_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"escrow".as_slice(), mint.as_ref()], &crate::id())
}

fn get_passes_supply_pda(owner: &Pubkey) -> (Pubkey, u8) {
    // seeds = [b"supply", passes_owner.key.as_ref()]
    Pubkey::find_program_address(&[b"supply".as_slice(), owner.as_ref()], &crate::id())
}

fn get_passes_balance_pda(owner: &Pubkey, buyer: &Pubkey) -> (Pubkey, u8) {
    // seeds = [b"balance", passes_owner.key.as_ref(), buyer.key.as_ref()]
    Pubkey::find_program_address(
        &[b"balance".as_slice(), owner.as_ref(), buyer.as_ref()],
        &crate::id(),
    )
}

fn create_and_fund_account(program_test: &mut ProgramTest, address: &Pubkey) {
    program_test.add_account(
        *address,
        account::Account {
            lamports: 1_000_000_000,
            ..account::Account::default()
        },
    );
}

pub async fn init_and_fund_token_account(
    ctx: &mut ProgramTestContext,
    mint: &Pubkey,
    owner: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Pubkey {
    let token_account_address = ctx
        .initialize_token_accounts(*mint, &[*owner])
        .await
        .unwrap()[0];

    ctx.mint_tokens(mint_authority, mint, &token_account_address, amount)
        .await
        .unwrap();

    eprintln!(
        "Init_and_fund_token_account: mint {}, owner {}, token_account_address {}, amount {}",
        mint, owner, token_account_address, amount
    );

    token_account_address
}

async fn init_passes(ctx: &mut ProgramTestContext, initializer: &Keypair, mint: &Pubkey) {
    // get pdas
    let (state, _) = get_state_pda();
    let (escrow_wallet, _) = get_escrow_pda(mint);
    let fee_token =
        anchor_spl::associated_token::get_associated_token_address(&initializer.pubkey(), mint);

    let args = instruction::Init {
        protocol_fee_pct: PROTOCOL_FEE_PCT,
        owner_fee_pct: OWNER_FEE_PCT,
    };
    let accounts = accounts::Init {
        admin: initializer.pubkey(),
        state,
        escrow_wallet,
        protocol_fee_token: fee_token,
        payment_mint: *mint,
        system_program: system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    };
    let res = execute_tx(ctx, accounts.to_account_metas(None), &args, initializer).await;
    assert_matches!(res, Ok(()));
}

fn init_mints(program_test: &mut ProgramTest, authority: &Pubkey) -> HashMap<String, MintInfo> {
    let mut mints = hashmap! {
        "usdc".to_string()  => MintInfo{ decimals: USDC_DECIMALS, pubkey: Pubkey::default() }
    };
    mints.iter_mut().for_each(|(_, info)| {
        info.pubkey = program_test.add_mint(None, info.decimals, authority).0
    });

    mints
}

async fn execute_tx<T: InstructionData>(
    ctx: &mut ProgramTestContext,
    accounts_meta: Vec<AccountMeta>,
    args: &T,
    payer: &Keypair,
) -> std::result::Result<(), BanksClientError> {
    let last_blockhash = ctx.last_blockhash;
    let banks_client = &mut ctx.banks_client;

    let ix = solana_sdk::instruction::Instruction {
        program_id: crate::id(),
        accounts: accounts_meta,
        data: args.data(),
    };

    let mut tx = solana_sdk::transaction::Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[payer], last_blockhash);

    banks_client.process_transaction(tx).await
}

async fn simulate_tx<T: InstructionData, U: BorshDeserialize>(
    ctx: &mut ProgramTestContext,
    accounts_meta: Vec<AccountMeta>,
    args: &T,
    payer: &Keypair,
) -> std::result::Result<U, BanksClientError> {
    let last_blockhash = ctx.last_blockhash;
    let banks_client = &mut ctx.banks_client;

    let ix = solana_sdk::instruction::Instruction {
        program_id: crate::id(),
        accounts: accounts_meta,
        data: args.data(),
    };

    let mut tx = solana_sdk::transaction::Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
    tx.sign(&[payer], last_blockhash);

    let result = banks_client.simulate_transaction(tx).await?;

    // Extract the returned data
    let mut return_data: Vec<u8> = result.simulation_details.unwrap().return_data.unwrap().data;

    let result_expected_len = std::mem::size_of::<U>();

    // Returned data doesn't contains leading zeros, need to re-add them before deserialization
    while return_data.len() < result_expected_len {
        return_data.push(0u8);
    }

    Ok(U::try_from_slice(return_data.as_slice()).unwrap())
}

async fn buy_passes(
    ctx: &mut ProgramTestContext,
    buyer: &Keypair,
    owner: &Pubkey,
    admin: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) {
    let signer = buyer;
    let buyer = &buyer.pubkey();

    // get pdas
    let (state, _) = get_state_pda();
    let (passes_supply, _) = get_passes_supply_pda(owner);
    let (passes_balance, _) = get_passes_balance_pda(owner, buyer);
    let (escrow_wallet, _) = get_escrow_pda(mint);
    eprintln!("escrow_wallet = {:#?}", escrow_wallet);
    let fee_token = anchor_spl::associated_token::get_associated_token_address(admin, mint);
    eprintln!("fee_token = {:#?}", fee_token);
    let buyer_wallet = anchor_spl::associated_token::get_associated_token_address(buyer, mint);
    eprintln!("buyer_wallet = {:#?}", buyer_wallet);
    let owner_fee_token = anchor_spl::associated_token::get_associated_token_address(owner, mint);
    eprintln!("owner_fee_token = {:#?}", owner_fee_token);

    let args = instruction::BuyPasses { amount };
    let accounts = accounts::BuyPasses {
        buyer: *buyer,
        passes_supply,
        passes_balance,
        state,
        owner_fee_token,
        escrow_wallet,
        passes_owner: *owner,
        payment_mint: *mint,
        protocol_fee_token: fee_token,
        buyer_wallet,
        system_program: system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    };
    let res = execute_tx(ctx, accounts.to_account_metas(None), &args, signer).await;
    assert_matches!(res, Ok(()));
}

async fn buy_passes_sol(
    ctx: &mut ProgramTestContext,
    buyer: &Keypair,
    owner: &Pubkey,
    admin: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) {
    let signer = buyer;
    let buyer = &buyer.pubkey();

    // get pdas
    let (state, _) = get_state_pda();
    let (passes_supply, _) = get_passes_supply_pda(owner);
    let (passes_balance, _) = get_passes_balance_pda(owner, buyer);

    let args = instruction::BuyPassesSol { amount };
    let accounts = accounts::BuyPassesSol {
        buyer: *buyer,
        passes_supply,
        passes_balance,
        state,
        passes_owner: *owner,
        admin: *admin,
        system_program: system_program::ID,
    };
    let res = execute_tx(ctx, accounts.to_account_metas(None), &args, signer).await;
    assert_matches!(res, Ok(()));
}

async fn sell_passes(
    ctx: &mut ProgramTestContext,
    seller: &Keypair,
    owner: &Pubkey,
    admin: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) {
    let signer = seller;
    let seller = &seller.pubkey();

    // get pdas
    let (state, _) = get_state_pda();
    let (passes_supply, _) = get_passes_supply_pda(owner);
    let (passes_balance, _) = get_passes_balance_pda(owner, seller);
    let (escrow_wallet, _) = get_escrow_pda(mint);
    eprintln!("escrow_wallet = {:#?}", escrow_wallet);
    let fee_token = anchor_spl::associated_token::get_associated_token_address(admin, mint);
    eprintln!("fee_token = {:#?}", fee_token);
    let seller_wallet = anchor_spl::associated_token::get_associated_token_address(seller, mint);
    eprintln!("seller_wallet = {:#?}", seller_wallet);
    let owner_fee_token = anchor_spl::associated_token::get_associated_token_address(owner, mint);
    eprintln!("owner_fee_token = {:#?}", owner_fee_token);

    let args = instruction::SellPasses { amount };
    let accounts = accounts::SellPasses {
        seller: *seller,
        passes_supply,
        passes_balance,
        state,
        owner_fee_token,
        escrow_wallet,
        passes_owner: *owner,
        payment_mint: *mint,
        protocol_fee_token: fee_token,
        seller_wallet,
        system_program: system_program::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    };
    let res = execute_tx(ctx, accounts.to_account_metas(None), &args, signer).await;
    assert_matches!(res, Ok(()));
}

pub async fn get_token_account(
    ctx: &mut ProgramTestContext,
    key: Pubkey,
) -> spl_token::state::Account {
    let banks_client = &mut ctx.banks_client;

    let raw_account = banks_client.get_account(key).await.unwrap().unwrap();

    spl_token::state::Account::unpack(&raw_account.data).unwrap()
}

pub async fn get_token_account_balance(ctx: &mut ProgramTestContext, key: Pubkey) -> u64 {
    get_token_account(ctx, key).await.amount
}

/*
macro_rules! get_pdas {
    ($owner:expr, $buyer:expr) => {
        let (state_pda, _) = Pubkey::find_program_address(&[b"state".as_slice()], &crate::id());

        let (escrow_pda, _) = Pubkey::find_program_address(&[b"escrow".as_slice()], &crate::id());

        // seeds = [b"supply", passes_owner.key.as_ref()]
        let (passes_supply_pda, _) = Pubkey::find_program_address(
            &[b"supply".as_slice(), $owner.key().as_ref()],
            &crate::id(),
        );

        // seeds = [b"balance", passes_owner.key.as_ref(), buyer.key.as_ref()]
        let (passes_balance_pda, _) = Pubkey::find_program_address(
            &[b"balance".as_slice(), $owner.as_ref(), $buyer.as_ref()],
            &crate::id(),
        );
    };
}
*/
