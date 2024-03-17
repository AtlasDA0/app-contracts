use std::vec;

use super::{app::StargazeApp, helpers::setup_block_time, msg::RaffleCodeIds};
use crate::common_setup::{
    contract_boxes::{
        contract_raffles, contract_sg721_base, contract_vending_factory, contract_vending_minter,
        custom_mock_app,
    },
    setup_minter::common::constants::{
        NOIS_PROXY_ADDR, OWNER_ADDR, RAFFLE_CONTRACT, RAFFLE_NAME, SG721_CONTRACT, VENDING_MINTER,
    },
};
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, Uint128};
use cw_multi_test::{BankSudo, Executor, SudoMsg};
use raffles::{
    msg::InstantiateMsg,
    state::{StakerFeeDiscount, ATLAS_DAO_STARGAZE_TREASURY},
};
use sg721::CollectionInfo;
use sg_std::NATIVE_DENOM;
use utils::state::NOIS_AMOUNT;
use vending_factory::{
    msg::{ExecuteMsg as SgVendingFactoryExecuteMsg, VendingMinterCreateMsg},
    state::{ParamsExtension, VendingMinterParams},
};

pub fn proper_raffle_instantiate() -> (StargazeApp, Addr, Addr) {
    let mut app = custom_mock_app();
    let chainid = app.block_info().chain_id.clone();
    setup_block_time(&mut app, 1647032400000000000, Some(10000), &chainid);

    let code_ids = raffle_template_code_ids(&mut app);

    // TODO: setup_factory_template
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

    // create raffle contract
    let raffle_contract_addr = app
        .instantiate_contract(
            code_ids.raffle_code_id,
            Addr::unchecked(OWNER_ADDR),
            &InstantiateMsg {
                name: RAFFLE_NAME.to_string(),
                nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
                nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM.to_string()),
                owner: Some(OWNER_ADDR.to_string()),
                fee_addr: Some(ATLAS_DAO_STARGAZE_TREASURY.to_owned()),
                minimum_raffle_duration: None,
                minimum_raffle_timeout: None,
                max_ticket_number: None,
                raffle_fee: Decimal::percent(50),
                creation_coins: vec![
                    coin(4, NATIVE_DENOM.to_string()),
                    coin(20, "ustars".to_string()),
                ]
                .into(),
                atlas_dao_nft_address: None,
                staker_fee_discount: StakerFeeDiscount {
                    discount: Decimal::zero(),
                    minimum_amount: Uint128::zero(),
                },
            },
            &[],
            "raffle",
            Some(Addr::unchecked(OWNER_ADDR).to_string()),
        )
        .unwrap();

    // fund raffle contract for nois_proxy fee
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: raffle_contract_addr.clone().to_string(),
            amount: vec![coin(100000000000u128, "ustars".to_string())],
        }
    }))
    .unwrap();
    // println!("raffle_contract_addr: {raffle_contract_addr}");
    // println!("factory_addr: {factory_addr}");
    // println!("{:#?}", res);

    (app, raffle_contract_addr, factory_addr)
}

pub fn configure_raffle_assets(
    app: &mut StargazeApp,
    owner_addr: Addr,
    sg_factory_addr: Addr,
    create_minter: bool,
) -> &mut StargazeApp {
    let router = app;
    let current_time = router.block_info().time;

    if create_minter {
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
                    code_id: 4,
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
    }

    // VENDING_MINTER is minter
    let mint_nft_tokens = router.execute_contract(
        owner_addr.clone(),
        Addr::unchecked(VENDING_MINTER),
        &vending_minter::msg::ExecuteMsg::Mint {},
        &[Coin {
            denom: NATIVE_DENOM.to_string(),
            amount: Uint128::new(100000u128),
        }],
    );
    assert!(mint_nft_tokens.is_ok());

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

    router
}

pub fn raffle_template_code_ids(router: &mut StargazeApp) -> RaffleCodeIds {
    let raffle_code_id = router.store_code(contract_raffles());
    let factory_code_id = router.store_code(contract_vending_factory());
    let minter_code_id = router.store_code(contract_vending_minter());
    let sg721_code_id = router.store_code(contract_sg721_base());

    RaffleCodeIds {
        raffle_code_id,
        minter_code_id,
        factory_code_id,
        sg721_code_id,
    }
}
