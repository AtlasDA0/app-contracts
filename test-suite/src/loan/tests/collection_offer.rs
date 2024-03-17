use cosmwasm_std::{coin, coins, Addr, BlockInfo, Coin, Decimal, Empty, Timestamp, Uint128};
use cw_multi_test::{BankSudo, Executor, SudoMsg};
use nft_loans_nc::{
    msg::{ExecuteMsg, InstantiateMsg, MultipleCollectionOffersResponse, QueryMsg},
    state::{CollateralInfo, LoanState, LoanTerms},
};
use sg721::CollectionInfo;
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;
use utils::state::{AssetInfo, Sg721Token};
use vending_factory::{
    msg::VendingMinterCreateMsg,
    state::{ParamsExtension, VendingMinterParams},
};

use crate::common_setup::{
    contract_boxes::{
        contract_nft_loans, contract_sg721_base, contract_vending_factory, contract_vending_minter,
        custom_mock_app,
    },
    setup_minter::common::constants::OWNER_ADDR,
};

const OFFERER_ADDR: &str = "offerer";
const DEPOSITOR_ADDR: &str = "depositor";
const BORROWER_ADDR: &str = "borrower";

const LISTING_FEE_NATIVE: u128 = 55;

// (App, loan_addr, factory_addr, minter_addr, nft_addr)
pub fn proper_instantiate() -> (StargazeApp, Addr, Addr, Addr, Addr) {
    // setup mock blockchain environment
    let mut app = custom_mock_app();
    let chainid = app.block_info().chain_id.clone();
    app.set_block(BlockInfo {
        height: 10000,
        time: Timestamp::from_nanos(1647032400000000000),
        chain_id: chainid,
    });

    // fund test account
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: OWNER_ADDR.to_string(),
            amount: vec![coin(100000000000u128, NATIVE_DENOM.to_string())],
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: OFFERER_ADDR.to_string(),
            amount: vec![coin(100000000000u128, NATIVE_DENOM.to_string())],
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: BORROWER_ADDR.to_string(),
            amount: vec![coin(100000000000u128, NATIVE_DENOM.to_string())],
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: DEPOSITOR_ADDR.to_string(),
            amount: vec![coin(100000000000u128, NATIVE_DENOM.to_string())],
        }
    }))
    .unwrap();

    // store wasm code for nft, minter , nft-loan
    let loan_code_id = app.store_code(contract_nft_loans());
    let factory_id = app.store_code(contract_vending_factory());
    let minter_id = app.store_code(contract_vending_minter());
    let sg721_id = app.store_code(contract_sg721_base());

    // setup nft minter
    let factory_addr = app
        .instantiate_contract(
            factory_id,
            Addr::unchecked(OWNER_ADDR),
            &vending_factory::msg::InstantiateMsg {
                params: VendingMinterParams {
                    code_id: minter_id,
                    allowed_sg721_code_ids: vec![sg721_id],
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
            loan_code_id,
            Addr::unchecked(OWNER_ADDR),
            &InstantiateMsg {
                name: "loan-with-insights".to_string(),
                owner: Some(Addr::unchecked(OWNER_ADDR).to_string()),
                treasury_addr: Addr::unchecked(OWNER_ADDR).to_string(),
                fee_rate: Decimal::percent(5),
                listing_fee_coins: vec![
                    coin(LISTING_FEE_NATIVE, NATIVE_DENOM.to_string()),
                    coin(45, "usstars".to_string()),
                ]
                .into(),
            },
            &[],
            "loans",
            Some(Addr::unchecked(OWNER_ADDR).to_string()),
        )
        .unwrap();

    let (minter_addr, nft_addr) =
        create_nft_collection(&mut app, factory_addr.clone(), nft_loan_addr.clone());

    (
        app,
        nft_loan_addr,
        factory_addr,
        Addr::unchecked(minter_addr),
        Addr::unchecked(nft_addr),
    )
}

pub fn create_nft_collection(
    app: &mut StargazeApp,
    factory_addr: Addr,
    nft_loan_addr: Addr,
) -> (String, String) {
    let current_time = app.block_info().time;

    // create nft minter
    let create_nft_minter = app.execute_contract(
        Addr::unchecked(OWNER_ADDR),
        factory_addr.clone(),
        &vending_factory::msg::ExecuteMsg::CreateMinter(VendingMinterCreateMsg {
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
                    creator: "creator".to_string(),
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

    let addresses = create_nft_minter
        .unwrap()
        .events
        .into_iter()
        .filter(|e| e.ty == "instantiate")
        .flat_map(|e| {
            e.attributes
                .into_iter()
                .filter(|a| a.key == "_contract_addr")
                .map(|a| a.value)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let minter_address = addresses[0].clone();
    let nft_address = addresses[1].clone();

    // VENDING_MINTER is minter
    // Minting tokens 63 and 65
    let _mint_nft_tokens = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            Addr::unchecked(minter_address.clone()),
            &vending_minter::msg::ExecuteMsg::Mint {},
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100000u128),
            }],
        )
        .unwrap();
    let _mint_nft_tokens_again = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            Addr::unchecked(minter_address.clone()),
            &vending_minter::msg::ExecuteMsg::Mint {},
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100000u128),
            }],
        )
        .unwrap();

    // token id 63
    let _grant_approval = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            Addr::unchecked(nft_address.clone()),
            &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                spender: nft_loan_addr.to_string(),
                token_id: "63".to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();
    // token id 65
    let _grant_approval = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            Addr::unchecked(nft_address.clone()),
            &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                spender: nft_loan_addr.to_string(),
                token_id: "65".to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

    (minter_address, nft_address)
}

#[test]
pub fn collection_offer_works() {
    let (mut app, nft_loan_addr, _factory_addr, _minter, nft) = proper_instantiate();

    let _collection_offer = app
        .execute_contract(
            Addr::unchecked(OFFERER_ADDR),
            nft_loan_addr.clone(),
            &ExecuteMsg::MakeCollectionOffer {
                collection: nft.to_string(),
                terms: LoanTerms {
                    principle: Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    },
                    interest: Uint128::new(50),
                    duration_in_blocks: 15,
                },
                comment: None,
            },
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();

    let _accept_collection_offer = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            nft_loan_addr.clone(),
            &ExecuteMsg::AcceptCollectionOffer {
                collection_offer_id: 1,
                token: AssetInfo::Sg721Token(Sg721Token {
                    address: nft.to_string(),
                    token_id: "63".to_string(),
                }),
            },
            &coins(LISTING_FEE_NATIVE, NATIVE_DENOM),
        )
        .unwrap();

    // We make sure the collateral exist and is in the right state
    let collateral: CollateralInfo = app
        .wrap()
        .query_wasm_smart(
            nft_loan_addr,
            &QueryMsg::CollateralInfo {
                borrower: OWNER_ADDR.to_string(),
                loan_id: 0,
            },
        )
        .unwrap();

    assert_eq!(collateral.state, LoanState::Started);
}

#[test]
pub fn query_works() {
    // We use 2 different collection offers
    // We make sure only the one that is interested pops up

    let (mut app, nft_loan_addr, factory_addr, _minter, nft) = proper_instantiate();

    // We create a second fee collection
    let (_minter_1, nft_1) =
        create_nft_collection(&mut app, factory_addr.clone(), nft_loan_addr.clone());

    let _collection_offer = app
        .execute_contract(
            Addr::unchecked(OFFERER_ADDR),
            nft_loan_addr.clone(),
            &ExecuteMsg::MakeCollectionOffer {
                collection: nft.to_string(),
                terms: LoanTerms {
                    principle: Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    },
                    interest: Uint128::new(50),
                    duration_in_blocks: 15,
                },
                comment: None,
            },
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();

    let _collection_offer_1 = app
        .execute_contract(
            Addr::unchecked(OFFERER_ADDR),
            nft_loan_addr.clone(),
            &ExecuteMsg::MakeCollectionOffer {
                collection: nft_1.to_string(),
                terms: LoanTerms {
                    principle: Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    },
                    interest: Uint128::new(50),
                    duration_in_blocks: 15,
                },
                comment: None,
            },
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();

    // We query the one collection or the other to make sure the query works

    let res: MultipleCollectionOffersResponse = app
        .wrap()
        .query_wasm_smart(
            &nft_loan_addr,
            &QueryMsg::CollectionOffers {
                collection: nft.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(res.next_offer, None);
    assert_eq!(res.offers.len(), 1);
    assert_eq!(
        res.offers[0].collection_offer_info.collection,
        nft.to_string()
    );

    let res: MultipleCollectionOffersResponse = app
        .wrap()
        .query_wasm_smart(
            &nft_loan_addr,
            &QueryMsg::CollectionOffers {
                collection: nft_1.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(res.next_offer, None);
    assert_eq!(res.offers.len(), 1);
    assert_eq!(
        res.offers[0].collection_offer_info.collection,
        nft_1.to_string()
    );
}
