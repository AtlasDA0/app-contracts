use crate::common_setup::app::StargazeApp;
use crate::common_setup::{
    contract_boxes::{
        contract_nft_loans, contract_sg721_base, contract_vending_factory, contract_vending_minter,
        custom_mock_app,
    },
    setup_minter::common::constants::{RAFFLE_CONTRACT, SG721_CONTRACT, VENDING_MINTER},
};
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, Uint128};
use cw_multi_test::Executor;
use nft_loans_nc::msg::InstantiateMsg as LoanInstantiateMsg;
use sg721::CollectionInfo;
use sg_std::NATIVE_DENOM;
use vending_factory::{
    msg::{ExecuteMsg as SgVendingFactoryExecuteMsg, VendingMinterCreateMsg},
    state::{ParamsExtension, VendingMinterParams},
};

use super::{
    helpers::setup_block_time,
    msg::LoanCodeIds,
    setup_minter::common::constants::{OWNER_ADDR, TREASURY_ADDR},
};

pub fn loan_template_code_ids(router: &mut StargazeApp) -> LoanCodeIds {
    let minter_code_id = router.store_code(contract_vending_minter());
    println!("minter_code_id: {minter_code_id}");

    let factory_code_id = router.store_code(contract_vending_factory());
    println!("factory_code_id: {factory_code_id}");

    let sg721_code_id = router.store_code(contract_sg721_base());
    println!("sg721_code_id: {sg721_code_id}");

    let loan_code_id = router.store_code(contract_nft_loans());
    println!("loan_code_id: {loan_code_id}");

    LoanCodeIds {
        minter_code_id,
        factory_code_id,
        sg721_code_id,
        loan_code_id,
    }
}

pub fn proper_loan_instantiate() -> (StargazeApp, Addr, Addr) {
    // setup mock blockchain environment
    let mut app = custom_mock_app();
    let chainid = app.block_info().chain_id.clone();
    setup_block_time(&mut app, 1647032400000000000, Some(10000), &chainid);
    // store wasm code for nft, minter , nft-loan
    let code_ids = loan_template_code_ids(&mut app);

    // setup nft minter
    let factory_addr = app
        .instantiate_contract(
            code_ids.factory_code_id,
            Addr::unchecked(OWNER_ADDR),
            &vending_factory::msg::InstantiateMsg {
                params: VendingMinterParams {
                    code_id: code_ids.minter_code_id,
                    allowed_sg721_code_ids: vec![code_ids.sg721_code_id],
                    frozen: false,
                    creation_fee: Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100000u128),
                    },
                    min_mint_price: Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100000u128),
                    },
                    mint_fee_bps: 10,
                    max_trading_offset_secs: 0,
                    extension: ParamsExtension {
                        max_token_limit: 1000,
                        max_per_address_limit: 20,
                        airdrop_mint_price: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100000u128),
                        },
                        airdrop_mint_fee_bps: 10,
                        shuffle_fee: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100000u128),
                        },
                    },
                },
            },
            &[],
            "factory",
            Some(OWNER_ADDR.to_string()),
        )
        .unwrap();

    // create nft-loan contract
    let nft_loan_addr = app
        .instantiate_contract(
            code_ids.loan_code_id,
            Addr::unchecked(OWNER_ADDR),
            &LoanInstantiateMsg {
                name: "loan-with-insights".to_string(),
                owner: Some(Addr::unchecked(OWNER_ADDR).to_string()),
                treasury_addr: Addr::unchecked(TREASURY_ADDR).to_string(),
                fee_rate: Decimal::percent(50),
                listing_fee_coins: vec![
                    coin(25, NATIVE_DENOM.to_string()),
                    coin(50, "uflix".to_string()),
                    coin(10, "ujuno".to_string()),
                ]
                .into(),
            },
            &[],
            "loans",
            Some(Addr::unchecked(OWNER_ADDR).to_string()),
        )
        .unwrap();

    (app, nft_loan_addr, factory_addr)
}

pub fn configure_loan_assets(
    app: &mut StargazeApp,
    owner_addr: Addr,
    sg_factory_addr: Addr,
) -> &mut StargazeApp {
    let router = app;
    let current_time = router.block_info().time;

    let _create_nft_minter = router.execute_contract(
        owner_addr.clone(),
        sg_factory_addr.clone(),
        &SgVendingFactoryExecuteMsg::CreateMinter(VendingMinterCreateMsg {
            init_msg: vending_factory::msg::VendingMinterInitMsgExtension {
                base_token_uri: "ipfs://aldkfjads".to_string(),
                payment_address: Some(OWNER_ADDR.to_string()),
                start_time: current_time,
                num_tokens: 100,
                mint_price: coin(Uint128::new(100000u128).u128(), NATIVE_DENOM),
                per_address_limit: 3,
                whitelist: None,
            },
            collection_params: sg2::msg::CollectionParams {
                code_id: 3,
                name: "Collection Name".to_string(),
                symbol: "COL".to_string(),
                info: CollectionInfo {
                    creator: owner_addr.to_string(),
                    description: String::from("Atlanauts"),
                    image: "https://example.com/image.png".to_string(),
                    external_link: Some("https://example.com/external.html".to_string()),
                    start_trading_time: None,
                    explicit_content: Some(false),
                    royalty_info: None,
                },
            },
        }),
        &[Coin {
            denom: NATIVE_DENOM.to_string(),
            amount: Uint128::new(100000u128),
        }],
    );
    // println!("{:#?}", create_nft_minter);

    // VENDING_MINTER is minter
    let mint1 = router.execute_contract(
        owner_addr.clone(),
        Addr::unchecked(VENDING_MINTER),
        &vending_minter::msg::ExecuteMsg::Mint {},
        &[Coin {
            denom: NATIVE_DENOM.to_string(),
            amount: Uint128::new(100000u128),
        }],
    );
    let mint2 = router.execute_contract(
        owner_addr.clone(),
        Addr::unchecked(VENDING_MINTER),
        &vending_minter::msg::ExecuteMsg::Mint {},
        &[Coin {
            denom: NATIVE_DENOM.to_string(),
            amount: Uint128::new(100000u128),
        }],
    );
    assert!((mint1.is_ok() || mint2.is_ok()));

    // token id 63
    let _grant_approval = router
        .execute_contract(
            owner_addr.clone(),
            Addr::unchecked(SG721_CONTRACT),
            &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                spender: RAFFLE_CONTRACT.to_string(),
                token_id: "63".to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

    // token id 34
    let _grant_approval = router
        .execute_contract(
            owner_addr.clone(),
            Addr::unchecked(SG721_CONTRACT),
            &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                spender: RAFFLE_CONTRACT.to_string(),
                token_id: "34".to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

    router
}
