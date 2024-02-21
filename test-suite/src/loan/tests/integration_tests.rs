#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, StdError, Timestamp, Uint128};
    use cw_multi_test::Executor;
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
    use utils::state::{AssetInfo, Locks, Sg721Token};

    use crate::{
        common_setup::{
            helpers::assert_error,
            setup_accounts_and_block::setup_accounts,
            setup_loan::{configure_loan_assets, proper_loan_instantiate},
            setup_minter::common::constants::OWNER_ADDR,
        },
        loan::setup::{execute_msg::create_loan_function, test_msgs::CreateLoanParams},
    };

    const TREASURY_ADDR: &str = "collector";
    const OFFERER_ADDR: &str = "offerer";
    const VENDING_MINTER: &str = "contract2";
    const SG721_CONTRACT: &str = "contract3";

    // TODO: setup function to create nft minter, mint & approve nfts
    #[test]
    fn integration_test_loans() {
        // setup test environment
        let (mut app, loan_addr, factory_addr) = proper_loan_instantiate();
        let (owner_address, one, _) = setup_accounts(&mut app);
        configure_loan_assets(&mut app, owner_address.clone(), factory_addr);
        let create_loan_params: CreateLoanParams<'_> = CreateLoanParams {
            app: &mut app,
            loan_contract_addr: loan_addr.clone(),
            owner_addr: owner_address.clone(),
        };
        create_loan_function(create_loan_params.into()).unwrap();

        // good list collaterals
        let [contract_bal_before, sender_bal_before, fee_bal_before]: [Uint128; 3] = [
            loan_addr.to_string(),
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
                loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "63".to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "34".to_string(),
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
                    amount: Uint128::new(25u128),
                }],
            )
            .unwrap();

        // confirm fees end up where they need to be
        let [contract_bal_after, sender_bal_after, fee_bal_after]: [Uint128; 3] = [
            loan_addr.to_string(),
            owner_address.to_string(),
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
        // sender should be 25 less
        assert_eq!(sender_bal_after, sender_bal_before - Uint128::new(25u128));
        // collector should have extra 25
        assert_eq!(fee_bal_after, fee_bal_before + Uint128::new(25u128));
        // query new collateral_offer
        let res: MultipleCollateralsResponse = app
            .wrap()
            .query_wasm_smart(
                loan_addr.clone(),
                &nft_loans_nc::msg::QueryMsg::Collaterals {
                    borrower: Addr::unchecked(OWNER_ADDR).to_string(),
                    start_after: None,
                    limit: None,
                    // filters: None,
                },
            )
            .unwrap();
        assert_eq!(
            res.collaterals[0],
            CollateralResponse {
                borrower: owner_address.to_string(),
                loan_id: 1,
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
                            token_id: "34".to_string(),
                        })
                    ],
                    list_date: Timestamp::from_nanos(1647032400000000000),
                    state: LoanState::Published,
                    offer_amount: 0,
                    active_offer: None,
                    start_block: None,
                    comment: Some("be water, my friend".to_string()),
                    loan_preview: None,
                },
                loan_state: LoanState::Published,
            }
        );

        // create loan_id 1
        let _deposit1 = app
            .execute_contract(
                owner_address.clone(),
                loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "34".to_string(),
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
                    amount: Uint128::new(25u128),
                }],
            )
            .unwrap();
        // println!("{:#?}", _deposit97);

        // confirm error if no collateral listing fee provided
        let bad_deposit_collateral_no_fee = app
            .execute_contract(
                owner_address.clone(),
                loan_addr.clone(),
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
                owner_address.clone(),
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
                &ExecuteMsg::ListCollaterals {
                    tokens: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "63".to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "34".to_string(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
        let bad_withdraw_collateral = app.execute_contract(
            Addr::unchecked("not-owner".to_string()),
            loan_addr.clone(),
            &ExecuteMsg::WithdrawCollaterals { loan_id: 0 },
            &[],
        );
        assert!(bad_withdraw_collateral.is_err());
        // LoanNotFound error due to msg.sender not being contract admin

        // no funds sent in offer
        let bad_make_offer_no_funds = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
                &ExecuteMsg::WithdrawCollaterals { loan_id: 1 },
                &[],
            )
            .unwrap();
        // println!("{:#?}", _withdraw97);
        // let res: ApprovalResponse = app
        //         .wrap()
        //         .query_wasm_smart(
        //             SG721_CONTRACT.to_string(),
        //             &Sg721QueryMsg::Approval { token_id: "63".to_string(), spender: loan_addr.to_string(), include_expired: None }
        //         )
        //         .unwrap();
        //     println!("{:#?}", res);
        // assert_eq!(res, "fee".to_string());

        // good make offfer
        let _good_make_offer = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
                &ExecuteMsg::AcceptOffer {
                    global_offer_id: 1.to_string(),
                },
                &[],
            )
            .unwrap();
        let res: OfferResponse = app
            .wrap()
            .query_wasm_smart(
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
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
                loan_addr.clone(),
                &ExecuteMsg::SetFeeRate {
                    fee_rate: Decimal::percent(69),
                },
                &[],
            )
            .unwrap();
        let res: Config = app
            .wrap()
            .query_wasm_smart(loan_addr.clone(), &QueryMsg::Config {})
            .unwrap();
        assert_eq!(res.fee_rate, Decimal::percent(69));

        // error if unauthorized to set owner
        let bad_set_owner = app
            .execute_contract(
                Addr::unchecked("not-admin".to_string()),
                loan_addr.clone(),
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
                loan_addr.clone(),
                &ExecuteMsg::SetOwner {
                    owner: "new-admin".to_string(),
                },
                &[],
            )
            .unwrap();
        let res: Config = app
            .wrap()
            .query_wasm_smart(loan_addr.clone(), &QueryMsg::Config {})
            .unwrap();
        assert_eq!(res.owner, "new-admin".to_string());
    }
}
