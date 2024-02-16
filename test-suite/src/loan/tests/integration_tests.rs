#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, StdError, Timestamp, Uint128};
    use cw_multi_test::Executor;
    use raffles::state::ATLAS_DAO_STARGAZE_TREASURY;
    use sg_std::NATIVE_DENOM;

    use sg721::CollectionInfo;
    use vending_factory::msg::VendingMinterCreateMsg;

    use nft_loans_nc::{
        error::ContractError,
        msg::{
            CollateralResponse, ExecuteMsg, MultipleCollateralsResponse, OfferResponse, QueryMsg,
        },
        state::{CollateralInfo, Config, LoanState, LoanTerms, OfferState},
    };
    use utils::state::{AssetInfo, Sg721Token};

    use crate::{
        common_setup::{
            contract_boxes::custom_mock_app,
            helpers::assert_error,
            setup_loan::proper_loan_instantiate,
            setup_minter::common::constants::{
                LOAN_INTEREST_TAX, LOAN_NAME, MINT_PRICE, MIN_COLLATERAL_LISTING, OWNER_ADDR,
            },
        },
        loan::setup::{execute_msg::instantate_loan_contract, test_msgs::InstantiateParams},
    };

    const TREASURY_ADDR: &str = "collector";
    const OFFERER_ADDR: &str = "offerer";
    const VENDING_MINTER: &str = "contract2";
    const SG721_CONTRACT: &str = "contract3";

    #[test]
    fn test_instantiate() {
        let mut app = custom_mock_app();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: LOAN_INTEREST_TAX,
            name: LOAN_NAME.into(),
        };
        instantate_loan_contract(params).unwrap();
    }

    #[test]
    fn test_instantiate_fee_rate() {
        let mut app = custom_mock_app();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: LOAN_INTEREST_TAX + Decimal::percent(60),
            name: LOAN_NAME.into(),
        };
        let res = instantate_loan_contract(params).unwrap_err();
        assert_eq!(
            res.root_cause().to_string(),
            "The fee_rate you provided is not greater than 0, or less than 1"
        );
    }

    #[test]
    fn test_instantiate_name() {
        let mut app = custom_mock_app();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: LOAN_INTEREST_TAX,
            name: "80808080808080808080808080808080808080808080808080808080808080808080808080808080808088080808080808080808080808080808080808080808080808080808080808080808080808080808080808".to_string(),
        };
        let res1 = instantate_loan_contract(params).unwrap_err();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: LOAN_INTEREST_TAX,
            name: "80".to_string(),
        };
        let res2 = instantate_loan_contract(params).unwrap_err();
        assert_eq!(res1.root_cause().to_string(), "Invalid Name");
        assert_eq!(res2.root_cause().to_string(), "Invalid Name");
    }

    #[test]
    fn test_config_query() {
        let mut app = custom_mock_app();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: LOAN_INTEREST_TAX,
            name: LOAN_NAME.into(),
        };
        let nft_loan_addr = instantate_loan_contract(params).unwrap();

        let query_config: Config = app
            .wrap()
            .query_wasm_smart(
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::QueryMsg::Config {},
            )
            .unwrap();
        assert_eq!(
            query_config,
            Config {
                name: LOAN_NAME.into(),
                owner: Addr::unchecked(OWNER_ADDR.to_string()),
                treasury_addr: Addr::unchecked(ATLAS_DAO_STARGAZE_TREASURY.to_string()),
                fee_rate: LOAN_INTEREST_TAX,
                global_offer_index: 0,
                listing_fee_coins: vec![
                    Coin::new(MIN_COLLATERAL_LISTING, NATIVE_DENOM),
                    Coin::new(MIN_COLLATERAL_LISTING, "usstars"),
                ]
            }
        );
    }

    #[test]
    fn test_update_contract_coverage() {
        let (mut app, nft_loan_addr, _) = proper_loan_instantiate();

        // errors
        let error_updating_listing_coins = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::ExecuteMsg::SetListingCoins {
                    listing_fee_coins: vec![coin(4, "uflix"), coin(5, "uscrt"), coin(6, "uatom")],
                },
                &[],
            )
            .unwrap_err();
        let error_updating_fee_destination = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::ExecuteMsg::SetFeeDestination {
                    treasury_addr: OWNER_ADDR.into(),
                },
                &[],
            )
            .unwrap_err();
        let error_updating_fee_percent = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::ExecuteMsg::SetFeeRate {
                    fee_rate: Decimal::percent(20),
                },
                &[],
            )
            .unwrap_err();

        assert_error(
            Err(error_updating_listing_coins),
            ContractError::Unauthorized {}.to_string(),
        );
        assert_error(
            Err(error_updating_fee_destination),
            ContractError::Unauthorized {}.to_string(),
        );
        assert_error(
            Err(error_updating_fee_percent),
            ContractError::Unauthorized {}.to_string(),
        );
        // good responses
        let _check_listing_coins_with_existing_coin = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::ExecuteMsg::SetListingCoins {
                    listing_fee_coins: vec![
                        coin(4, "uflix"),
                        coin(5, "uscrt"),
                        coin(6, "uatom"),
                        coin(7, "ustars"),
                    ],
                },
                &[],
            )
            .unwrap();
        let _check_set_fee_rate = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::ExecuteMsg::SetFeeRate {
                    fee_rate: Decimal::percent(10),
                },
                &[],
            )
            .unwrap();
        let _check_set_fee_rate = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::ExecuteMsg::SetFeeDestination {
                    treasury_addr: ATLAS_DAO_STARGAZE_TREASURY.into(),
                },
                &[],
            )
            .unwrap();

        let res: Config = app
            .wrap()
            .query_wasm_smart(
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::QueryMsg::Config {},
            )
            .unwrap();

        // println!("{:#?}", res);
        assert_eq!(
            res,
            Config {
                name: LOAN_NAME.into(),
                owner: Addr::unchecked(OWNER_ADDR.to_string()),
                treasury_addr: Addr::unchecked(ATLAS_DAO_STARGAZE_TREASURY.to_string()),
                fee_rate: Decimal::percent(10),
                listing_fee_coins: vec![
                    coin(4, "uflix"),
                    coin(5, "uscrt"),
                    coin(6, "uatom"),
                    coin(7, "ustars"),
                ],
                global_offer_index: 0
            }
        )
    }

    // TODO: setup function to create nft minter, mint & approve nfts
    #[test]
    fn integration_test_loans() {
        // setup test environment
        let (mut app, nft_loan_addr, factory_addr) = proper_loan_instantiate();
        let current_time = app.block_info().time.clone();

        // create nft minter
        let _create_nft_minter = app.execute_contract(
            Addr::unchecked(OWNER_ADDR),
            factory_addr.clone(),
            &vending_factory::msg::ExecuteMsg::CreateMinter {
                0: VendingMinterCreateMsg {
                    init_msg: vending_factory::msg::VendingMinterInitMsgExtension {
                        base_token_uri: "ipfs://aldkfjads".to_string(),
                        payment_address: Some(OWNER_ADDR.to_string()),
                        start_time: current_time.clone(),
                        num_tokens: 100,
                        mint_price: coin(Uint128::new(100000u128).u128(), NATIVE_DENOM),
                        per_address_limit: 3,
                        whitelist: None,
                    },
                    collection_params: sg2::msg::CollectionParams {
                        code_id: 3, // cw721 code id
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
                },
            },
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100000u128),
            }],
        );
        // println!("{:#?}", _create_nft_minter);

        // VENDING_MINTER is minter
        let _mint63 = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                Addr::unchecked(VENDING_MINTER),
                &vending_minter::msg::ExecuteMsg::Mint {},
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100000u128),
                }],
            )
            .unwrap();
        // println!("{:#?}", _mint63);

        // mint nft token-id 65
        let _mint65 = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                Addr::unchecked(VENDING_MINTER),
                &vending_minter::msg::ExecuteMsg::Mint {},
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100000u128),
                }],
            )
            .unwrap();

        // mint nft token-id 97
        let _mint97 = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                Addr::unchecked(VENDING_MINTER),
                &vending_minter::msg::ExecuteMsg::Mint {},
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100000u128),
                }],
            )
            .unwrap();
        // println!("{:#?}", _mint97);

        // grant approval token id 63
        let _grant_approval = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                Addr::unchecked(SG721_CONTRACT),
                &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                    spender: nft_loan_addr.to_string(),
                    token_id: "63".to_string(),
                    expires: None,
                },
                &[],
            )
            .unwrap();

        // grant approval token id 65
        let _grant_approval = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                Addr::unchecked(SG721_CONTRACT),
                &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                    spender: nft_loan_addr.to_string(),
                    token_id: "65".to_string(),
                    expires: None,
                },
                &[],
            )
            .unwrap();

        // grant approval token id 97
        let _grant_approval = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                Addr::unchecked(SG721_CONTRACT),
                &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                    spender: nft_loan_addr.to_string(),
                    token_id: "97".to_string(),
                    expires: None,
                },
                &[],
            )
            .unwrap();

        // good list collaterals
        let [contract_bal_before, sender_bal_before, fee_bal_before]: [Uint128; 3] = [
            nft_loan_addr.to_string(),
            OWNER_ADDR.to_string(),
            TREASURY_ADDR.to_string(),
        ]
        .into_iter()
        .map(|x| {
            app.wrap()
                .query_balance(&x, NATIVE_DENOM.to_string())
                .unwrap()
                .amount
        })
        .collect::<Vec<Uint128>>()
        .try_into() // Try to convert Vec<Uint128> into [Uint128; 3]
        .unwrap();

        // create loan_id 0
        let _good_list_collaterals = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "63".to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "65".to_string(),
                        }),
                    ],
                    terms: Some(LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100),
                        },
                        interest: Uint128::new(50),
                        duration_in_blocks: 15,
                    }),
                    comment: Some("be water, my friend".to_string()),
                    loan_preview: None,
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(55u128),
                }],
            )
            .unwrap();

        // confirm fees end up where they need to be
        let [contract_bal_after, sender_bal_after, fee_bal_after]: [Uint128; 3] = [
            nft_loan_addr.to_string(),
            OWNER_ADDR.to_string(),
            TREASURY_ADDR.to_string(),
        ]
        .into_iter()
        .map(|x| {
            app.wrap()
                .query_balance(&x, NATIVE_DENOM.to_string())
                .unwrap()
                .amount
        })
        .collect::<Vec<Uint128>>()
        .try_into() // Try to convert Vec<Uint128> into [Uint128; 3]
        .unwrap();

        // contract shouldnt hold fees
        assert_eq!(contract_bal_after, contract_bal_before);
        // sender should be 50 less
        assert_eq!(sender_bal_after, sender_bal_before - Uint128::new(55u128));
        // collector should have extra 50
        assert_eq!(fee_bal_after, fee_bal_before + Uint128::new(55u128));
        // query new collateral_offer
        let res: MultipleCollateralsResponse = app
            .wrap()
            .query_wasm_smart(
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::QueryMsg::Collaterals {
                    borrower: Addr::unchecked(OWNER_ADDR).to_string(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(
            res.collaterals[0],
            CollateralResponse {
                borrower: Addr::unchecked(OWNER_ADDR).to_string(),
                loan_id: 0,
                collateral: CollateralInfo {
                    terms: Some(LoanTerms {
                        principle: coin(100, "ustars"),
                        interest: Uint128::new(50u128),
                        duration_in_blocks: 15,
                    }),
                    associated_assets: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "63".to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "65".to_string(),
                        })
                    ],
                    list_date: Timestamp::from_nanos(1647032400000000000),
                    state: LoanState::Published,
                    offer_amount: 0,
                    active_offer: None,
                    start_block: None,
                    comment: Some("be water, my friend".to_string()),
                    loan_preview: None,
                }
            }
        );

        // create loan_id 1
        let _deposit97 = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "97".to_string(),
                    })],
                    terms: Some(LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100),
                        },
                        interest: Uint128::new(50),
                        duration_in_blocks: 15,
                    }),
                    comment: Some("be water, my friend".to_string()),
                    loan_preview: None,
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(55u128),
                }],
            )
            .unwrap();
        // println!("{:#?}", _deposit97);

        // confirm error if no collateral listing fee provided
        let bad_deposit_collateral_no_fee = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "63".to_string(),
                    })],
                    terms: Some(LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100),
                        },
                        interest: Uint128::new(15),
                        duration_in_blocks: 15,
                    }),
                    comment: Some("be water, my friend".to_string()),
                    loan_preview: None,
                },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(bad_deposit_collateral_no_fee),
            ContractError::DepositFeeError {}.to_string(),
        );

        // confirm error on multiple coins sent, one being correct coin type,
        // but incorrect amount required. We also send an incorrect coin type
        //  with the correct amount expected, to ensure the contract does not signal false
        // positive for miscalculation.
        let bad_deposit_collateral_wrong_denoms = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "63".to_string(),
                    })],
                    terms: Some(LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100),
                        },
                        interest: Uint128::new(15),
                        duration_in_blocks: 15,
                    }),
                    comment: Some("be water, my friend".to_string()),
                    loan_preview: None,
                },
                &[
                    Coin {
                        denom: "uscrt".to_string(),
                        amount: Uint128::new(50u128),
                    },
                    Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(50u128),
                    },
                ],
            )
            .unwrap_err();
        assert_error(
            Err(bad_deposit_collateral_wrong_denoms),
            ContractError::DepositFeeError {}.to_string(),
        );

        // confirm contract error on too much collateral listing fee provided.
        let bad_deposit_collateral_too_much_fee = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "63".to_string(),
                    })],
                    terms: Some(LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100),
                        },
                        interest: Uint128::new(15),
                        duration_in_blocks: 15,
                    }),
                    comment: Some("be water, my friend".to_string()),
                    loan_preview: None,
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(80u128),
                }],
            )
            .unwrap_err();
        assert_error(
            Err(bad_deposit_collateral_too_much_fee),
            ContractError::DepositFeeError {}.to_string(),
        );

        // too little collateral listing fee provided
        let bad_deposit_collateral_no_fee = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "63".to_string(),
                    })],
                    terms: Some(LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100),
                        },
                        interest: Uint128::new(15),
                        duration_in_blocks: 15,
                    }),
                    comment: Some("be water, my friend".to_string()),
                    loan_preview: None,
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(40u128),
                }],
            )
            .unwrap_err();
        assert_error(
            Err(bad_deposit_collateral_no_fee),
            ContractError::DepositFeeError {}.to_string(),
        );

        // ensure info.sender is owner of nfts
        let bad_deposit_collateral_not_owner = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "63".to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "65".to_string(),
                        }),
                    ],
                    terms: Some(LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100),
                        },
                        interest: Uint128::new(50),
                        duration_in_blocks: 15,
                    }),
                    comment: Some("Real living is living for others".to_string()),
                    loan_preview: None,
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100u128),
                }],
            )
            .unwrap_err();
        assert_error(
            Err(bad_deposit_collateral_not_owner),
            ContractError::SenderNotOwner {}.to_string(),
        );

        // good modify collateral
        let _good_modify_collateral = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::ModifyCollaterals {
                    loan_id: 0,
                    terms: None,
                    comment: Some(
                        "Knowledge will give you power, but character respect".to_string(),
                    ),
                    loan_preview: None,
                },
                &[],
            )
            .unwrap();

        let res: CollateralInfo = app
            .wrap()
            .query_wasm_smart(
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::QueryMsg::CollateralInfo {
                    borrower: Addr::unchecked(OWNER_ADDR).to_string(),
                    loan_id: 0,
                },
            )
            .unwrap();
        assert_eq!(
            res.comment,
            Some("Knowledge will give you power, but character respect".to_string())
        );

        // bad modify collateral
        let bad_modify_collateral = app
            .execute_contract(
                Addr::unchecked("not-admin"),
                nft_loan_addr.clone(),
                &ExecuteMsg::ModifyCollaterals {
                    loan_id: 0,
                    terms: None,
                    comment: Some("Showing off is the fools idea of glory".to_string()),
                    loan_preview: None,
                },
                &[],
            )
            .unwrap_err();

        // LoanNotFound error due to msg.sender not being contract admin
        assert_error(
            Err(bad_modify_collateral),
            ContractError::LoanNotFound {}.to_string(),
        );

        // bad withdraw collateral
        let bad_withdraw_collateral = app
            .execute_contract(
                Addr::unchecked("not-owner".to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::WithdrawCollaterals { loan_id: 0 },
                &[],
            )
            .unwrap_err();

        // LoanNotFound error due to msg.sender not being contract admin
        assert_error(Err(bad_withdraw_collateral), StdError::NotFound {
                    kind: "type: nft_loans_nc::state::CollateralInfo; key: [00, 0F, 63, 6F, 6C, 6C, 61, 74, 65, 72, 61, 6C, 5F, 69, 6E, 66, 6F, 00, 09, 6E, 6F, 74, 2D, 6F, 77, 6E, 65, 72, 00, 00, 00, 00, 00, 00, 00, 00]"
                    .to_string()}.to_string());

        // no funds sent in offer
        let bad_make_offer_no_funds = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::MakeOffer {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    terms: LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(50),
                        },
                        interest: Uint128::new(15),
                        duration_in_blocks: 15,
                    },
                    comment: Some("Obey the principles without being bound by them".to_string()),
                },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(bad_make_offer_no_funds),
            ContractError::MultipleCoins {}.to_string(),
        );

        // too little funds sent in offer
        let bad_make_offer_too_little_funds = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::MakeOffer {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    terms: LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(50),
                        },
                        interest: Uint128::new(15),
                        duration_in_blocks: 15,
                    },
                    comment: Some("Obey the principles without being bound by them".to_string()),
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(40),
                }],
            )
            .unwrap_err();
        assert_error(
            Err(bad_make_offer_too_little_funds),
            ContractError::FundsDontMatchTerms {}.to_string(),
        );

        // too much funds sent in offer
        let bad_make_offer_too_much_funds = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::MakeOffer {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    terms: LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(50),
                        },
                        interest: Uint128::new(15),
                        duration_in_blocks: 15,
                    },
                    comment: Some("Obey the principles without being bound by them".to_string()),
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(80),
                }],
            )
            .unwrap_err();
        assert_error(
            Err(bad_make_offer_too_much_funds),
            ContractError::FundsDontMatchTerms {}.to_string(),
        );

        // not enough funds sent
        let bad_accept_loan_not_enough_funds_sent = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::AcceptLoan {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    comment: Some("A quick temper will make a fool of you soon enough".to_string()),
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(50),
                }],
            )
            .unwrap_err();
        assert_error(
            Err(bad_accept_loan_not_enough_funds_sent),
            ContractError::FundsDontMatchTerms {}.to_string(),
        );

        // too much funds sent
        let bad_accept_loan_too_much_funds_sent = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::AcceptLoan {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    comment: Some("A quick temper will make a fool of you soon enough".to_string()),
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(150),
                }],
            )
            .unwrap_err();
        assert_error(
            Err(bad_accept_loan_too_much_funds_sent),
            ContractError::FundsDontMatchTerms {}.to_string(),
        );

        // 0 funds sent
        let bad_accept_loan_no_funds_sent = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::AcceptLoan {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    comment: Some("A quick temper will make a fool of you soon enough".to_string()),
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(90),
                }],
            )
            .unwrap_err();
        assert_error(
            Err(bad_accept_loan_no_funds_sent),
            ContractError::FundsDontMatchTerms {}.to_string(),
        );

        // not owner of offer_id
        let bad_accept_offer = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                nft_loan_addr.clone(),
                &ExecuteMsg::AcceptOffer {
                    global_offer_id: 1.to_string(),
                },
                &[],
            )
            .unwrap_err();
        // println!("{:#?}", bad_accept_offer);
        assert_error(
            Err(bad_accept_offer),
            StdError::GenericErr {
                msg: "invalid offer".to_string(),
            }
            .to_string(),
        );

        // not owner of offer_id
        let bad_cancel_offer = app
            .execute_contract(
                Addr::unchecked("not-offerer"),
                nft_loan_addr.clone(),
                &ExecuteMsg::CancelOffer {
                    global_offer_id: 1.to_string(),
                },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(bad_cancel_offer),
            StdError::GenericErr {
                msg: "invalid offer".to_string(),
            }
            .to_string(),
        );

        // good withdraw collateral
        let _withdraw97 = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::WithdrawCollaterals { loan_id: 1 },
                &[],
            )
            .unwrap();
        // println!("{:#?}", _withdraw97);
        // let res: ApprovalResponse = app
        //         .wrap()
        //         .query_wasm_smart(
        //             SG721_CONTRACT.to_string(),
        //             &Sg721QueryMsg::Approval { token_id: "63".to_string(), spender: nft_loan_addr.to_string(), include_expired: None }
        //         )
        //         .unwrap();
        //     println!("{:#?}", res);
        // assert_eq!(res, "fee".to_string());

        // good make offfer
        let _good_make_offer = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::MakeOffer {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    terms: LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100),
                        },
                        interest: Uint128::new(50),
                        duration_in_blocks: 15,
                    },
                    comment: Some("Obey the principles without being bound by them".to_string()),
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100),
                }],
            )
            .unwrap();
        let res: OfferResponse = app
            .wrap()
            .query_wasm_smart(
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::QueryMsg::OfferInfo {
                    global_offer_id: 1.to_string(),
                },
            )
            .unwrap();
        assert_eq!(res.global_offer_id, 1.to_string());

        // good accept offer
        let _good_accept_offer = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::AcceptOffer {
                    global_offer_id: 1.to_string(),
                },
                &[],
            )
            .unwrap();
        let res: OfferResponse = app
            .wrap()
            .query_wasm_smart(
                nft_loan_addr.clone(),
                &QueryMsg::OfferInfo {
                    global_offer_id: 1.to_string(),
                },
            )
            .unwrap();
        // assert the offer state is now accepted
        assert_eq!(res.offer_info.state, OfferState::Accepted);

        // repay borrowed funds
        let _good_repay_borrowed_funds = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::RepayBorrowedFunds { loan_id: 0 },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(150),
                }],
            )
            .unwrap();
        // println!("{:#?}", _good_repay_borrowed_funds);
        let res: CollateralInfo = app
            .wrap()
            .query_wasm_smart(
                nft_loan_addr.clone(),
                &QueryMsg::CollateralInfo {
                    borrower: Addr::unchecked(OWNER_ADDR).to_string(),
                    loan_id: 0,
                },
            )
            .unwrap();
        assert_eq!(res.state, LoanState::Ended);

        // unauthorized update fee destination addr
        let bad_set_fee_distributor = app
            .execute_contract(
                Addr::unchecked("not-admin".to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetFeeDestination {
                    treasury_addr: "not-admin".to_string(),
                },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(bad_set_fee_distributor),
            ContractError::Unauthorized {}.to_string(),
        );

        // error if unauthorized to set fee rate
        let bad_set_fee_rate = app
            .execute_contract(
                Addr::unchecked("not-admin".to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetFeeRate {
                    fee_rate: Decimal::percent(69),
                },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(bad_set_fee_rate),
            ContractError::Unauthorized {}.to_string(),
        );

        // good set fee rate
        let _set_fee_rate = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetFeeRate {
                    fee_rate: Decimal::percent(69),
                },
                &[],
            )
            .unwrap();
        let res: Config = app
            .wrap()
            .query_wasm_smart(nft_loan_addr.clone(), &QueryMsg::Config {})
            .unwrap();
        assert_eq!(res.fee_rate, Decimal::percent(69));

        // error if unauthorized to set owner
        let bad_set_owner = app
            .execute_contract(
                Addr::unchecked("not-admin".to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetOwner {
                    owner: "not-admin".to_string(),
                },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(bad_set_owner),
            ContractError::Unauthorized {}.to_string(),
        );

        // good set owner
        let _good_set_owner = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetOwner {
                    owner: "new-admin".to_string(),
                },
                &[],
            )
            .unwrap();
        let res: Config = app
            .wrap()
            .query_wasm_smart(nft_loan_addr.clone(), &QueryMsg::Config {})
            .unwrap();
        assert_eq!(res.owner, "new-admin".to_string());
    }
}
