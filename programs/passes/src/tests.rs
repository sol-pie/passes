#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use std::collections::HashMap;

use anchor_lang::{
    prelude::{borsh::BorshDeserialize, *},
    system_program, InstructionData, Owner,
};
use assert_matches::*;
use bonfida_test_utils::ProgramTestContextExt;
use bonfida_test_utils::ProgramTestExt;
use maplit::hashmap;
use solana_program::program_pack::Pack;
use solana_program_test::{tokio, BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::{self, ReadableAccount},
    instruction::AccountMeta,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

use crate::{
    accounts::{self},
    instruction::{self},
    state::{Config, PassesBalance, PassesSupply},
    ONE_USDC, USDC_DECIMALS,
};
use utils::*;

// TODO bps?
// const PROTOCOL_FEE_USDC: u64 = 10_000; // 1% = 0.01*10^6
// const OWNER_FEE_USDC: u8 = 10_000; // 1% = 0.01*10^6
// const PROTOCOL_FEE_SOL: u8 = 10_000_000; // 1% = 0.01*10^9
// const OWNER_FEE_SOL: u64 = 10_000_000; // 1% = 0.01*10^9
const PROTOCOL_FEE_BPS: u64 = 100; // 100bps = 1%
const OWNER_FEE_BPS: u64 = 100; // 100bps = 1%
const TICKER: &str = "usdc";

#[derive(Debug)]
pub struct MintInfo {
    pub decimals: u8,
    pub pubkey: Pubkey,
}

#[tokio::test]
async fn test_init() {
    let (mut ctx, initializer, _, _, mint) = setup().await;

    init_passes(
        &mut ctx,
        &initializer,
        &mint,
        PROTOCOL_FEE_BPS,
        OWNER_FEE_BPS,
    )
    .await;

    // get pda
    let (escrow_token_wallet, _) = get_escrow_token_wallet_pda(&mint);
    let (escrow_sol_wallet, _) = get_escrow_sol_wallet_pda();
    let protocol_fee_wallet =
        anchor_spl::associated_token::get_associated_token_address(&initializer.pubkey(), &mint);

    // check protocol fee percent
    let (config_pda, _) = get_config_pda();
    let config: Config = get_account(&mut ctx, config_pda).await;
    assert_eq!(config.admin, initializer.pubkey());
    assert_eq!(config.payment_mint, mint);
    assert_eq!(config.escrow_sol_wallet, escrow_sol_wallet);
    assert_eq!(config.escrow_token_wallet, escrow_token_wallet);
    assert_eq!(config.owner_fee_bps, OWNER_FEE_BPS);
    assert_eq!(config.protocol_fee_bps, PROTOCOL_FEE_BPS);
    assert_eq!(config.protocol_fee_token_wallet, protocol_fee_wallet);
}

#[tokio::test]
async fn test_get_price() {
    let (mut ctx, initializer, _, _, _) = setup().await;

    // calc and get price for token (e.g. usdc)
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

    // calc and get price for sol
    let args = instruction::GetPriceSol {
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
    assert_matches!(res, Ok(5625000_u64));
}

#[tokio::test]
async fn test_set_fees_pct() {
    let (mut ctx, initializer, _, _, mint) = setup().await;

    // get pda
    let (config_pda, _) = get_config_pda();

    init_passes(
        &mut ctx,
        &initializer,
        &mint,
        PROTOCOL_FEE_BPS,
        OWNER_FEE_BPS,
    )
    .await;

    // set protocol fee percent
    let args = instruction::SetProtocolFeeBps { fee_bps: 11111111 };
    let accounts = accounts::SetFeePercent {
        admin: initializer.pubkey(),
        config: config_pda,
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
    let args = instruction::SetOwnerFeeBps { fee_bps: 2222222 };
    let accounts = accounts::SetFeePercent {
        admin: initializer.pubkey(),
        config: config_pda,
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
    let config: Config = get_account(&mut ctx, config_pda).await;
    assert_eq!(config.protocol_fee_bps, 11111111);
    assert_eq!(config.owner_fee_bps, 2222222);
}

#[tokio::test]
async fn test_set_fee_dst() {
    let (mut ctx, initializer, _, _, mint) = setup().await;

    // get pda
    let (config_pda, _) = get_config_pda();

    init_passes(
        &mut ctx,
        &initializer,
        &mint,
        PROTOCOL_FEE_BPS,
        OWNER_FEE_BPS,
    )
    .await;

    // set protocol fee dst
    let protocol_fee_wallet =
        anchor_spl::associated_token::get_associated_token_address(&initializer.pubkey(), &mint);
    let args = instruction::SetProtocolFeeDst {};
    let accounts = accounts::SetProtocolFeeDst {
        admin: initializer.pubkey(),
        config: config_pda,
        protocol_fee_wallet,
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
    let config: Config = get_account(&mut ctx, config_pda).await;
    assert_eq!(config.payment_mint, mint);
    assert_eq!(config.protocol_fee_token_wallet, protocol_fee_wallet);
}

#[tokio::test]
async fn test_issue_passes() {
    let (mut ctx, initializer, buyer, owner, mint) = setup().await;

    init_passes(
        &mut ctx,
        &initializer,
        &mint,
        PROTOCOL_FEE_BPS,
        OWNER_FEE_BPS,
    )
    .await;

    issue_passes(&mut ctx, &owner, &mint).await;

    let (passes_supply_pda, _) = get_passes_supply_pda(&owner.pubkey());
    let passes_supply: PassesSupply = get_account(&mut ctx, passes_supply_pda).await;
    assert_eq!(passes_supply.amount, 1);

    let (passes_balance_pda, _) = get_passes_balance_pda(&owner.pubkey(), &owner.pubkey());
    let passes_balance: PassesBalance = get_account(&mut ctx, passes_balance_pda).await;
    assert_eq!(passes_balance.amount, 1);
}

#[tokio::test]
async fn test_buy_passes_w_usdc() {
    let (mut ctx, initializer, buyer, owner, mint) = setup().await;

    init_passes(
        &mut ctx,
        &initializer,
        &mint,
        PROTOCOL_FEE_BPS,
        OWNER_FEE_BPS,
    )
    .await;

    issue_passes(&mut ctx, &owner, &mint).await;

    // buyer buy passes
    buy_passes(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mint,
        10,
    )
    .await;

    // buyer buy passes second time
    buy_passes(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mint,
        1,
    )
    .await;

    // check escrow fund
    let (escrow_wallet, _) = get_escrow_token_wallet_pda(&mint);
    let account = get_token_account(&mut ctx, escrow_wallet).await;
    assert_eq!(account.amount, 2406250 + 756250);

    // check protocol fee wallet
    let protocol_fee_wallet =
        anchor_spl::associated_token::get_associated_token_address(&initializer.pubkey(), &mint);
    let account = get_token_account(&mut ctx, protocol_fee_wallet).await;
    assert_eq!(account.amount, 24063 + 7563);

    // check owner fee wallet
    let owner_fee_wallet =
        anchor_spl::associated_token::get_associated_token_address(&owner.pubkey(), &mint);
    let account = get_token_account(&mut ctx, owner_fee_wallet).await;
    assert_eq!(account.amount, 24063 + 7563);

    // check total owner's pass supply
    let (passes_supply_pda, _) = get_passes_supply_pda(&owner.pubkey());
    let passes_supply: PassesSupply = get_account(&mut ctx, passes_supply_pda).await;
    assert_eq!(passes_supply.amount, 12);

    // check byuer's pass balance of owner's pass
    let (passes_balance_pda, _) = get_passes_balance_pda(&owner.pubkey(), &buyer.pubkey());
    let passes_balance: PassesBalance = get_account(&mut ctx, passes_balance_pda).await;
    assert_eq!(passes_balance.amount, 11);

    // check buyer balance after purchase and ...
    let buyer_wallet =
        anchor_spl::associated_token::get_associated_token_address(&buyer.pubkey(), &mint);
    let account = get_token_account(&mut ctx, buyer_wallet).await;
    assert_eq!(
        account.amount,
        5_u64 * ONE_USDC - 2406250 - 24063 - 24063 - 756250 - 7563 - 7563
    );
}

#[tokio::test]
async fn test_sell_passes_w_usdc() {
    let (mut ctx, initializer, buyer, owner, mint) = setup().await;

    init_passes(
        &mut ctx,
        &initializer,
        &mint,
        PROTOCOL_FEE_BPS,
        OWNER_FEE_BPS,
    )
    .await;

    issue_passes(&mut ctx, &owner, &mint).await;

    // buyer buy next passes
    buy_passes(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mint,
        10,
    )
    .await;

    // buyer sell part of their passes
    sell_passes(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mint,
        5,
    )
    .await;

    // check total owner's pass supply
    let (passes_supply_pda, _) = get_passes_supply_pda(&owner.pubkey());
    let passes_supply: PassesSupply = get_account(&mut ctx, passes_supply_pda).await;
    assert_eq!(passes_supply.amount, 6);

    // check byuer's pass balance of owner's pass
    let (passes_balance_pda, _) = get_passes_balance_pda(&owner.pubkey(), &buyer.pubkey());
    let passes_balance: PassesBalance = get_account(&mut ctx, passes_balance_pda).await;
    assert_eq!(passes_balance.amount, 5);

    // check protocol fee wallet
    let protocol_fee_wallet =
        anchor_spl::associated_token::get_associated_token_address(&initializer.pubkey(), &mint);
    let account = get_token_account(&mut ctx, protocol_fee_wallet).await;
    // assert_eq!(account.amount, 24062 + 20625);
    assert_eq!(account.amount, 24063 + 20625);

    // check owner fee wallet
    let owner_fee_wallet =
        anchor_spl::associated_token::get_associated_token_address(&owner.pubkey(), &mint);
    let account = get_token_account(&mut ctx, owner_fee_wallet).await;
    // assert_eq!(account.amount, 24062 + 20625);
    assert_eq!(account.amount, 24063 + 20625);

    // check buyer balance after purchase and ...
    let buyer_wallet =
        anchor_spl::associated_token::get_associated_token_address(&buyer.pubkey(), &mint);
    let account = get_token_account(&mut ctx, buyer_wallet).await;
    assert_eq!(
        account.amount,
        5_u64 * ONE_USDC - 2406250 - 24063 - 24063 + 2062500 - 20625 - 20625
    );
}

#[tokio::test]
async fn test_buy_passes_w_sol() {
    let (mut ctx, initializer, buyer, owner, mint) = setup().await;

    // get all pda s
    let (escrow_wallet, _) = get_escrow_sol_wallet_pda();
    let (passes_supply_pda, _) = get_passes_supply_pda(&owner.pubkey());
    let (passes_balance_pda, _) = get_passes_balance_pda(&owner.pubkey(), &buyer.pubkey());

    init_passes(
        &mut ctx,
        &initializer,
        &mint,
        PROTOCOL_FEE_BPS,
        OWNER_FEE_BPS,
    )
    .await;

    issue_passes(&mut ctx, &owner, &mint).await;

    let escrow_wallet_lamports_before = get_lamports(&mut ctx, &escrow_wallet).await;
    let protocol_fee_wallet_lamports_before = get_lamports(&mut ctx, &initializer.pubkey()).await;
    let owner_lamports_before = get_lamports(&mut ctx, &owner.pubkey()).await;

    // buyer buy passes
    buy_passes_sol(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mint,
        10,
    )
    .await;

    let passes_supply: PassesSupply = get_account(&mut ctx, passes_supply_pda).await;
    assert_eq!(passes_supply.amount, 11);

    let passes_balance: PassesBalance = get_account(&mut ctx, passes_balance_pda).await;
    assert_eq!(passes_balance.amount, 10);

    assert_eq!(
        get_lamports(&mut ctx, &escrow_wallet).await,
        escrow_wallet_lamports_before + 240_625_000
    );

    assert_eq!(
        get_lamports(&mut ctx, &initializer.pubkey()).await,
        protocol_fee_wallet_lamports_before + 2_406_250
    );

    assert_eq!(
        get_lamports(&mut ctx, &owner.pubkey()).await,
        owner_lamports_before + 2_406_250
    );
}

#[tokio::test]
async fn test_sell_passes_w_sol() {
    let (mut ctx, initializer, buyer, owner, mint) = setup().await;

    // get all pda s
    let (escrow_wallet, _) = get_escrow_sol_wallet_pda();
    let (passes_supply_pda, _) = get_passes_supply_pda(&owner.pubkey());
    let (passes_balance_pda, _) = get_passes_balance_pda(&owner.pubkey(), &buyer.pubkey());

    init_passes(
        &mut ctx,
        &initializer,
        &mint,
        PROTOCOL_FEE_BPS,
        OWNER_FEE_BPS,
    )
    .await;

    issue_passes(&mut ctx, &owner, &mint).await;

    let escrow_wallet_lamports_before = get_lamports(&mut ctx, &escrow_wallet).await;
    let protocol_fee_wallet_lamports_before = get_lamports(&mut ctx, &initializer.pubkey()).await;
    let owner_lamports_before = get_lamports(&mut ctx, &owner.pubkey()).await;

    // buyer buy passes
    buy_passes_sol(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mint,
        10,
    )
    .await;

    // buyer sell part of their passes
    sell_passes_sol(
        &mut ctx,
        &buyer,
        &owner.pubkey(),
        &initializer.pubkey(),
        &mint,
        5,
    )
    .await;

    // check total owner's pass supply
    let (passes_supply_pda, _) = get_passes_supply_pda(&owner.pubkey());
    let passes_supply: PassesSupply = get_account(&mut ctx, passes_supply_pda).await;
    assert_eq!(passes_supply.amount, 6);

    // check byuer's pass balance of owner's pass
    let (passes_balance_pda, _) = get_passes_balance_pda(&owner.pubkey(), &buyer.pubkey());
    let passes_balance: PassesBalance = get_account(&mut ctx, passes_balance_pda).await;
    assert_eq!(passes_balance.amount, 5);

    assert_eq!(
        get_lamports(&mut ctx, &escrow_wallet).await,
        escrow_wallet_lamports_before + 240_625_000 - 206_250_000
    );

    assert_eq!(
        get_lamports(&mut ctx, &initializer.pubkey()).await,
        protocol_fee_wallet_lamports_before + 2_406_250 + 2_062_500
    );

    assert_eq!(
        get_lamports(&mut ctx, &owner.pubkey()).await,
        owner_lamports_before + 2_406_250 + 2_062_500
    );
}

mod utils {
    use super::*;

    pub async fn setup() -> (ProgramTestContext, Keypair, Keypair, Keypair, Pubkey) {
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

        (ctx, initializer, buyer, owner, mints[TICKER].pubkey)
    }

    pub fn get_config_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"config".as_slice()], &crate::id())
    }

    pub fn get_escrow_token_wallet_pda(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"escrow".as_slice(), mint.as_ref()], &crate::id())
    }

    pub fn get_escrow_sol_wallet_pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"escrow".as_slice()], &crate::id())
    }

    pub fn get_passes_supply_pda(owner: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"supply".as_slice(), owner.as_ref()], &crate::id())
    }

    pub fn get_passes_balance_pda(owner: &Pubkey, buyer: &Pubkey) -> (Pubkey, u8) {
        // seeds = [b"balance", passes_owner.key.as_ref(), buyer.key.as_ref()]
        Pubkey::find_program_address(
            &[b"balance".as_slice(), owner.as_ref(), buyer.as_ref()],
            &crate::id(),
        )
    }

    pub fn create_and_fund_account(program_test: &mut ProgramTest, address: &Pubkey) {
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

    pub async fn init_passes(
        ctx: &mut ProgramTestContext,
        initializer: &Keypair,
        mint: &Pubkey,
        protocol_fee_bps: u64,
        owner_fee_bps: u64,
    ) {
        // get pdas
        let (config, _) = get_config_pda();
        let (escrow_token_wallet, _) = get_escrow_token_wallet_pda(mint);
        let (escrow_sol_wallet, _) = get_escrow_sol_wallet_pda();
        let protocol_fee_wallet =
            anchor_spl::associated_token::get_associated_token_address(&initializer.pubkey(), mint);

        let args = instruction::Init {
            protocol_fee_bps,
            owner_fee_bps,
        };
        let accounts = accounts::Init {
            admin: initializer.pubkey(),
            config,
            escrow_token_wallet,
            escrow_sol_wallet,
            protocol_fee_wallet,
            payment_mint: *mint,
            system_program: system_program::ID,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        };
        let res = execute_tx(ctx, accounts.to_account_metas(None), &args, initializer).await;
        assert_matches!(res, Ok(()));
    }

    pub fn init_mints(
        program_test: &mut ProgramTest,
        authority: &Pubkey,
    ) -> HashMap<String, MintInfo> {
        let mut mints = hashmap! {
            "usdc".to_string()  => MintInfo{ decimals: USDC_DECIMALS, pubkey: Pubkey::default() }
        };
        mints.iter_mut().for_each(|(_, info)| {
            info.pubkey = program_test.add_mint(None, info.decimals, authority).0
        });

        mints
    }

    pub async fn issue_passes(ctx: &mut ProgramTestContext, owner: &Keypair, mint: &Pubkey) {
        let passes_supply = get_passes_supply_pda(&owner.pubkey()).0;
        let passes_balance = get_passes_balance_pda(&owner.pubkey(), &owner.pubkey()).0;
        let config = get_config_pda().0;
        let owner_fee_wallet =
            anchor_spl::associated_token::get_associated_token_address(&owner.pubkey(), mint);

        let args = instruction::IssuePasses { amount: 1 };
        let accounts = accounts::IssuePasses {
            owner: owner.pubkey(),
            passes_supply,
            passes_balance,
            config,
            owner_fee_wallet,
            payment_mint: *mint,
            system_program: anchor_lang::system_program::ID,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        };

        let res = execute_tx(ctx, accounts.to_account_metas(None), &args, owner).await;
        assert_matches!(res, Ok(()));
    }

    pub async fn buy_passes(
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
        let (config, _) = get_config_pda();
        let (passes_supply, _) = get_passes_supply_pda(owner);
        let (passes_balance, _) = get_passes_balance_pda(owner, buyer);
        let (escrow_wallet, _) = get_escrow_token_wallet_pda(mint);
        let protocol_fee_wallet =
            anchor_spl::associated_token::get_associated_token_address(admin, mint);
        let buyer_wallet = anchor_spl::associated_token::get_associated_token_address(buyer, mint);
        let owner_fee_wallet =
            anchor_spl::associated_token::get_associated_token_address(owner, mint);
        // eprintln!("owner_fee_token = {:#?}", owner_fee_wallet);
        // eprintln!("buyer_wallet = {:#?}", buyer_wallet);
        // eprintln!("fee_token = {:#?}", protocol_fee_wallet);
        // eprintln!("escrow_wallet = {:#?}", escrow_wallet);

        let args = instruction::BuyPasses { amount };
        let accounts = accounts::BuyPasses {
            buyer: *buyer,
            passes_supply,
            passes_balance,
            config,
            owner_fee_wallet,
            escrow_wallet,
            passes_owner: *owner,
            payment_mint: *mint,
            protocol_fee_wallet,
            buyer_wallet,
            system_program: system_program::ID,
            token_program: anchor_spl::token::ID,
        };
        let res = execute_tx(ctx, accounts.to_account_metas(None), &args, signer).await;
        assert_matches!(res, Ok(()));
    }

    pub async fn buy_passes_sol(
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
        let (config, _) = get_config_pda();
        let (passes_supply, _) = get_passes_supply_pda(owner);
        let (passes_balance, _) = get_passes_balance_pda(owner, buyer);
        let (escrow_wallet, _) = get_escrow_sol_wallet_pda();

        let args = instruction::BuyPassesSol { amount };
        let accounts = accounts::BuyPassesSol {
            buyer: *buyer,
            passes_supply,
            passes_balance,
            config,
            escrow_wallet,
            protocol_fee_wallet: *admin,
            passes_owner: *owner,
            system_program: system_program::ID,
        };
        let res = execute_tx(ctx, accounts.to_account_metas(None), &args, signer).await;
        assert_matches!(res, Ok(()));
    }

    pub async fn sell_passes(
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
        let (config, _) = get_config_pda();
        let (passes_supply, _) = get_passes_supply_pda(owner);
        let (passes_balance, _) = get_passes_balance_pda(owner, seller);
        let (escrow_wallet, _) = get_escrow_token_wallet_pda(mint);
        let protocol_fee_wallet =
            anchor_spl::associated_token::get_associated_token_address(admin, mint);
        let seller_wallet =
            anchor_spl::associated_token::get_associated_token_address(seller, mint);
        let owner_fee_wallet =
            anchor_spl::associated_token::get_associated_token_address(owner, mint);
        // eprintln!("owner_fee_token = {:#?}", owner_fee_wallet);
        // eprintln!("buyer_wallet = {:#?}", buyer_wallet);
        // eprintln!("fee_token = {:#?}", protocol_fee_wallet);
        // eprintln!("escrow_wallet = {:#?}", escrow_wallet);

        let args = instruction::SellPasses { amount };
        let accounts = accounts::SellPasses {
            seller: *seller,
            passes_supply,
            passes_balance,
            config,
            owner_fee_wallet,
            escrow_wallet,
            passes_owner: *owner,
            payment_mint: *mint,
            protocol_fee_wallet,
            seller_wallet,
            system_program: system_program::ID,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        };
        let res = execute_tx(ctx, accounts.to_account_metas(None), &args, signer).await;
        assert_matches!(res, Ok(()));
    }

    pub async fn sell_passes_sol(
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
        let (config, _) = get_config_pda();
        let (passes_supply, _) = get_passes_supply_pda(owner);
        let (passes_balance, _) = get_passes_balance_pda(owner, seller);
        let (escrow_wallet, _) = get_escrow_sol_wallet_pda();

        let args = instruction::SellPassesSol { amount };
        let accounts = accounts::SellPassesSol {
            seller: *seller,
            passes_supply,
            passes_balance,
            config,
            escrow_wallet,
            passes_owner: *owner,
            protocol_fee_wallet: *admin,
            system_program: system_program::ID,
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

    pub async fn get_account<T: anchor_lang::AccountDeserialize>(
        ctx: &mut ProgramTestContext,
        key: Pubkey,
    ) -> T {
        let banks_client = &mut ctx.banks_client;

        let account = banks_client.get_account(key).await.unwrap().unwrap();

        T::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub async fn get_lamports(ctx: &mut ProgramTestContext, key: &Pubkey) -> u64 {
        let banks_client = &mut ctx.banks_client;

        let account = banks_client.get_account(*key).await.unwrap().unwrap();

        account.lamports
    }

    pub async fn execute_tx<T: InstructionData>(
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

        let mut tx =
            solana_sdk::transaction::Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
        tx.sign(&[payer], last_blockhash);

        banks_client.process_transaction(tx).await
    }

    pub async fn simulate_tx<T: InstructionData, U: BorshDeserialize>(
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

        let mut tx =
            solana_sdk::transaction::Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
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
}

/*
macro_rules! get_pdas {
    ($owner:expr, $buyer:expr) => {
        let (state_pda, _) = Pubkey::find_program_address(&[b"config".as_slice()], &crate::id());

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
