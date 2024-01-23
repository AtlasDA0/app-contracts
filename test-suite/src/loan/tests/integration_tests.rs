#[cfg(test)]
mod tests {
    use cosmwasm_std::{StdError, coin, Addr, BlockInfo, Coin, Decimal, Empty, Timestamp, Uint128};
    use sg_std::NATIVE_DENOM;
    use cw_multi_test::{BankSudo, Executor, SudoMsg};
    use sg_multi_test::StargazeApp;

    use sg721::CollectionInfo;
    use vending_factory::{
        msg::VendingMinterCreateMsg,
        state::{ParamsExtension, VendingMinterParams},
    };
    
    use utils::state::{AssetInfo, Sg721Token};
    use nft_loans::{
        error::ContractError,
        msg::{CollateralResponse, MultipleCollateralsResponse, OfferResponse, QueryMsg, ExecuteMsg, InstantiateMsg},
        state::{CollateralInfo, LoanState, OfferState, Config, LoanTerms},
    };
    

    use crate::common_setup::{contract_boxes::{
        contract_nft_loans, contract_sg721_base, contract_vending_factory, contract_vending_minter,
        custom_mock_app,
    }, helpers::assert_error};

    const OWNER_ADDR: &str = "fee";
    const OFFERER_ADDR: &str = "offerer";
    const VENDING_MINTER: &str = "contract2";
    const SG721_CONTRACT: &str = "contract3";

    pub fn proper_instantiate() -> (StargazeApp, Addr, Addr) {
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
                        code_id: minter_id.clone(),
                        allowed_sg721_code_ids: vec![sg721_id.clone()],
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
                    fee_distributor: Addr::unchecked(OWNER_ADDR).to_string(),
                    fee_rate: Decimal::percent(5),
                    deposit_fee_denom: vec!["usstars".to_string(), NATIVE_DENOM.to_string()],
                    deposit_fee_amount: 50,
                },
                &[],
                "loans",
                Some(Addr::unchecked(OWNER_ADDR).to_string()),
            )
            .unwrap();

        (app, nft_loan_addr, factory_addr)
    }

    #[test]
    fn init() {

            let (mut app, nft_loan_addr, factory_addr) = proper_instantiate();

            let current_time = app.block_info().time.clone();

            // query contract config
            let query_config: Config = app
                .wrap()
                .query_wasm_smart(nft_loan_addr.clone(), &nft_loans::msg::QueryMsg::Config {})
                .unwrap();
            assert_eq!(query_config.owner, Addr::unchecked("fee"));

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
                            code_id: 4,
                            name: "Collection Name".to_string(),
                            symbol: "COL".to_string(),
                            info: CollectionInfo {
                                creator: "creator".to_string(),
                                description: String::from("Atlanauts"),
                                image: "https://example.com/image.png".to_string(),
                                external_link: Some(
                                    "https://example.com/external.html".to_string(),
                                ),
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

            // VENDING_MINTER is minter
            let mint41 = app
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
            let mint56 = app
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

            let mint61 = app
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

        println!("{:#?}", mint61);

            // token id 41
            let _grant_approval = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    Addr::unchecked(SG721_CONTRACT),
                    &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                        spender: nft_loan_addr.to_string(),
                        token_id: "41".to_string(),
                        expires: None,
                    },
                    &[],
                )
                .unwrap();
            // token id 56
            let _grant_approval = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    Addr::unchecked(SG721_CONTRACT),
                    &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                        spender: nft_loan_addr.to_string(),
                        token_id: "56".to_string(),
                        expires: None,
                    },
                    &[],
                )
                .unwrap();
            // token id 61
            let _grant_approval = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    Addr::unchecked(SG721_CONTRACT),
                    &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                        spender: nft_loan_addr.to_string(),
                        token_id: "61".to_string(),
                        expires: None,
                    },
                    &[],
                )
                .unwrap();


            // good deposit collaterals
            let _good_deposit_collaterals = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    nft_loan_addr.clone(),
                    &ExecuteMsg::DepositCollaterals {
                        tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "41".to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "56".to_string(),
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
                        amount: Uint128::new(50u128),
                    }],
                )
                .unwrap();

            let res: MultipleCollateralsResponse = app
                .wrap()
                .query_wasm_smart(
                    nft_loan_addr.clone(),
                    &nft_loans::msg::QueryMsg::Collaterals {
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
                        associated_assets: vec![AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "41".to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "56".to_string(),
                        })],
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

            let deposit61 = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::DepositCollaterals {
                    tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "61".to_string(),
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
                    amount: Uint128::new(50u128),
                }],
            )
            .unwrap();

            // bad deposit collateral
            let bad_deposit_collateral_no_fee = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    nft_loan_addr.clone(),
                    &ExecuteMsg::DepositCollaterals {
                        tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "41".to_string(),
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
                ContractError::NoDepositFeeProvided {}.to_string(),
            );

            // ensure info.sender is owner of nfts
        let deposit_collateral_not_owner = app
        .execute_contract(
            Addr::unchecked(OFFERER_ADDR),
            nft_loan_addr.clone(),
            &ExecuteMsg::DepositCollaterals {
                tokens: vec![
                    AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "41".to_string(),
                    }),
                    AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "56".to_string(),
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
    assert_error(Err(deposit_collateral_not_owner), ContractError::SenderNotOwner {}.to_string());

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
                    &nft_loans::msg::QueryMsg::CollateralInfo {
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
            assert_error(Err(bad_withdraw_collateral), StdError::NotFound { 
                kind: "type: nft_loans::state::CollateralInfo; key: [00, 0F, 63, 6F, 6C, 6C, 61, 74, 65, 72, 61, 6C, 5F, 69, 6E, 66, 6F, 00, 09, 6E, 6F, 74, 2D, 6F, 77, 6E, 65, 72, 00, 00, 00, 00, 00, 00, 00, 00]"
                .to_string() }.to_string());

            // bad make offfer
            let bad_make_offer = app
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
                        comment: Some(
                            "Obey the principles without being bound by them".to_string(),
                        ),
                    },
                    &[],
                )
                .unwrap_err();
            assert_error(Err(bad_make_offer), ContractError::MultipleCoins {}.to_string());

            // bad accept loan
            let bad_accept_loan = app
                .execute_contract(
                    Addr::unchecked(OFFERER_ADDR.to_string()),
                    nft_loan_addr.clone(),
                    &ExecuteMsg::AcceptLoan {
                        borrower: OWNER_ADDR.to_string(),
                        loan_id: 0,
                        comment: Some(
                            "A quick temper will make a fool of you soon enough".to_string(),
                        ),
                    },
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(50),
                    }],
                )
                .unwrap_err();
            assert_error(Err(bad_accept_loan), ContractError::FundsDontMatchTerms {}.to_string());

            // bad accept offer
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
            assert_error(Err(bad_accept_offer), StdError::GenericErr { msg: "invalid offer".to_string() }.to_string());

            // bad cancel offer
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
            assert_error(Err(bad_cancel_offer), StdError::GenericErr { msg: "invalid offer".to_string() }.to_string());


            // // good withdraw collateral
            let withdraw61 = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR.to_string()),
                    nft_loan_addr.clone(),
                    &ExecuteMsg::WithdrawCollaterals { loan_id: 1 },
                    &[],
                )
                .unwrap();
            println!("{:#?}", withdraw61);

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
                        comment: Some(
                            "Obey the principles without being bound by them".to_string(),
                        ),
                    },
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    }],
                )
                .unwrap();
            let res: OfferResponse = app.wrap().query_wasm_smart(
                nft_loan_addr.clone(),
                 &nft_loans::msg::QueryMsg::OfferInfo { global_offer_id: 1.to_string() }).unwrap();
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
            let res: OfferResponse = app.wrap().query_wasm_smart(
                nft_loan_addr.clone(),
            &QueryMsg::OfferInfo { global_offer_id: 1.to_string()}).unwrap();
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
            let res: CollateralInfo = app.wrap().query_wasm_smart(
                nft_loan_addr.clone(),
                 &QueryMsg::CollateralInfo { 
                    borrower: Addr::unchecked(OWNER_ADDR).to_string(),
                     loan_id: 0,
                    }).unwrap();
            assert_eq!(res.state,LoanState::Ended);


            // bad fee distributor
            let bad_set_fee_distributor = app.execute_contract(
                Addr::unchecked("not-admin".to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetFeeDistributor { fee_depositor: "not-admin".to_string() },
                &[]).unwrap_err();
            assert_error(Err(bad_set_fee_distributor), ContractError::Unauthorized {}.to_string());

            // bad set fee rate
            let bad_set_fee_rate = app.execute_contract(
                Addr::unchecked("not-admin".to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetFeeRate { fee_rate: Decimal::percent(69) }, 
                &[]).unwrap_err();
            assert_error(Err(bad_set_fee_rate), ContractError::Unauthorized {}.to_string());

            // good set fee rate
            let _set_fee_rate = app.execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetFeeRate { fee_rate: Decimal::percent(69) }, 
                &[]).unwrap();
            let res: Config = app.wrap().query_wasm_smart(
                nft_loan_addr.clone(),
                &QueryMsg::Config {}).unwrap();
            assert_eq!(res.fee_rate, Decimal::percent(69));

            // bad set owner
            let bad_set_owner = app.execute_contract(
                Addr::unchecked("not-admin".to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetOwner { owner: "not-admin".to_string() },
                &[]).unwrap_err();
            assert_error(Err(bad_set_owner), ContractError::Unauthorized {}.to_string());

            // good set owner
            let _good_set_owner = app.execute_contract(
                Addr::unchecked(OWNER_ADDR),
                nft_loan_addr.clone(),
                &ExecuteMsg::SetOwner { owner: "new-admin".to_string() },
                &[]).unwrap();
            let res: Config = app.wrap().query_wasm_smart(
                nft_loan_addr.clone(),
                &QueryMsg::Config {}).unwrap();
            assert_eq!(res.owner,"new-admin".to_string());

        }
    }

