#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, BlockInfo, Coin, Decimal, Timestamp, Uint128};
    use cw_multi_test::Executor;
    use nft_loans::msg::InstantiateMsg;
    use sg_multi_test::StargazeApp;
    use sg_std::NATIVE_DENOM;
    use vending_factory::state::{ParamsExtension, VendingMinterParams};

    use crate::common_setup::contract_boxes::{
        contract_nft_loans, contract_sg721_base, contract_vending_factory, contract_vending_minter,
        custom_mock_app,
    };

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

    // certain functions may impact other functions, if present during testing.
    // TODO: update test workflow to prevent this issue
    mod init {
        use cosmwasm_std::{coin, Addr, Empty};
        use cw_multi_test::{BankSudo, SudoMsg};
        use nft_loans::{
            msg::ExecuteMsg,
            state::{Config, LoanTerms},
        };
        use sg721::CollectionInfo;
        use utils::state::{AssetInfo, Sg721Token};
        use vending_factory::msg::VendingMinterCreateMsg;

        use super::*;

        #[test]
        fn can_init() {
            let (mut app, nft_loan_addr, factory_addr) = proper_instantiate();

            // fund test account
            let current_time = app.block_info().time.clone();
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

            // query contract config
            let query_config: Config = app
                .wrap()
                .query_wasm_smart(nft_loan_addr.clone(), &nft_loans::msg::QueryMsg::Config {})
                .unwrap();
            assert_eq!(query_config.owner, Addr::unchecked("fee"));

            // create nft minter
            let create_nft_minter = app.execute_contract(
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
            println!("{:#?}", create_nft_minter);

            // VENDING_MINTER is minter
            let mint_nft_tokens = app
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
            println!("{:#?}", mint_nft_tokens);

            // token id 41
            let grant_approval = app
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
            println!("{:#?}", grant_approval);

            // good deposit collateral
            let good_deposit_collateral = app
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
                            interest: Uint128::new(50),
                            duration_in_blocks: 15,
                        }),
                        comment: Some("be water, my friend".to_string()),
                        loan_preview: None,
                    },
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100u128),
                    }],
                )
                .unwrap();
            println!("{:#?}", good_deposit_collateral);

            // bad deposit collateral
            // let bad_deposit_collateral_no_fee = app
            //     .execute_contract(
            //         Addr::unchecked(OWNER_ADDR),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::DepositCollaterals {
            //             tokens: vec![AssetInfo::Sg721Token(Sg721Token {
            //                 address: SG721_CONTRACT.to_string(),
            //                 token_id: "41".to_string(),
            //             })],
            //             terms: Some(LoanTerms {
            //                 principle: Coin {
            //                     denom: NATIVE_DENOM.to_string(),
            //                     amount: Uint128::new(100),
            //                 },
            //                 interest: Uint128::new(15),
            //                 duration_in_blocks: 15,
            //             }),
            //             comment: Some("be water, my friend".to_string()),
            //             loan_preview: None,
            //         },
            //         &[],
            //     )
            //     .unwrap_err();
            // println!("{:#?}", bad_deposit_collateral_no_fee);

            // good modify collateral
            // let good_modify_collateral = app
            //     .execute_contract(
            //         Addr::unchecked(OWNER_ADDR),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::ModifyCollaterals {
            //             loan_id: 0,
            //             terms: None,
            //             comment: Some(
            //                 "Knowledge will give you power, but character respect".to_string(),
            //             ),
            //             loan_preview: None,
            //         },
            //         &[],
            //     )
            //     .unwrap();
            // println!("{:#?}", good_modify_collateral);

            // bad modify collateral
            // let bad_modify_collateral = app
            //     .execute_contract(
            //         Addr::unchecked("not-admin"),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::ModifyCollaterals {
            //             loan_id: 0,
            //             terms: None,
            //             comment: Some("Showing off is the fools idea of glory".to_string()),
            //             loan_preview: None,
            //         },
            //         &[],
            //     )
            //     .unwrap_err();
            // println!("{:#?}", bad_modify_collateral);

            // bad withdraw collateral
            // let bad_withdraw_collateral = app
            //     .execute_contract(
            //         Addr::unchecked("not-owner".to_string()),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::WithdrawCollaterals { loan_id: 0 },
            //         &[],
            //     )
            //     .unwrap_err();
            // println!("{:#?}", bad_withdraw_collateral);

            // // good withdraw collateral
            // let good_withdraw_collateral = app
            //     .execute_contract(
            //         Addr::unchecked(OWNER_ADDR.to_string()),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::WithdrawCollaterals { loan_id: 0 },
            //         &[],
            //     )
            //     .unwrap();
            // println!("{:#?}", good_withdraw_collateral);

            // good make offfer
            let good_make_offer = app
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
            println!("{:#?}", good_make_offer);

            // bad make offfer
            // let bad_make_offer = app
            //     .execute_contract(
            //         Addr::unchecked(OFFERER_ADDR.to_string()),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::MakeOffer {
            //             borrower: OWNER_ADDR.to_string(),
            //             loan_id: 0,
            //             terms: LoanTerms {
            //                 principle: Coin {
            //                     denom: NATIVE_DENOM.to_string(),
            //                     amount: Uint128::new(50),
            //                 },
            //                 interest: Uint128::new(15),
            //                 duration_in_blocks: 15,
            //             },
            //             comment: Some(
            //                 "Obey the principles without being bound by them".to_string(),
            //             ),
            //         },
            //         &[],
            //     )
            //     .unwrap_err();
            // println!("{:#?}", bad_make_offer);

            // accept loan
            // let good_accept_loan = app
            //     .execute_contract(
            //         Addr::unchecked(OFFERER_ADDR.to_string()),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::AcceptLoan {
            //             borrower: OWNER_ADDR.to_string(),
            //             loan_id: 0,
            //             comment: Some("Real living is living for others".to_string()),
            //         },
            //         &[Coin {
            //             denom: NATIVE_DENOM.to_string(),
            //             amount: Uint128::new(100),
            //         }],
            //     )
            //     .unwrap();
            // println!("{:#?}", good_accept_loan);

            // bad accept loan
            // let bad_accept_loan = app
            //     .execute_contract(
            //         Addr::unchecked(OFFERER_ADDR.to_string()),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::AcceptLoan {
            //             borrower: OWNER_ADDR.to_string(),
            //             loan_id: 0,
            //             comment: Some(
            //                 "A quick temper will make a fool of you soon enough".to_string(),
            //             ),
            //         },
            //         &[Coin {
            //             denom: NATIVE_DENOM.to_string(),
            //             amount: Uint128::new(50),
            //         }],
            //     )
            //     .unwrap_err();
            // println!("{:#?}", bad_accept_loan);

            // bad accept offer
            // let bad_accept_offer = app
            //     .execute_contract(
            //         Addr::unchecked("not-owner"),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::AcceptOffer {
            //             global_offer_id: 1.to_string(),
            //         },
            //         &[],
            //     )
            //     .unwrap_err();
            // println!("{:#?}", bad_accept_offer);

            // good accept offer
            let good_accept_offer = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    nft_loan_addr.clone(),
                    &ExecuteMsg::AcceptOffer {
                        global_offer_id: 1.to_string(),
                    },
                    &[],
                )
                .unwrap();
            println!("{:#?}", good_accept_offer);

            // good cancel offer
            // let good_cancel_offer = app
            //     .execute_contract(
            //         Addr::unchecked(OFFERER_ADDR),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::CancelOffer {
            //             global_offer_id: 1.to_string(),
            //         },
            //         &[],
            //     )
            //     .unwrap();
            // println!("{:#?}", good_cancel_offer);

            // bad cancel offer
            // let bad_cancel_offer = app
            //     .execute_contract(
            //         Addr::unchecked("not-offerer"),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::CancelOffer {
            //             global_offer_id: 1.to_string(),
            //         },
            //         &[],
            //     )
            //     .unwrap_err();
            // println!("{:#?}", bad_cancel_offer);

            // good refuse offer
            // let good_refuse_offer = app.
            // execute_contract(
            //     Addr::unchecked(OWNER_ADDR),
            //     nft_loan_addr.clone(),
            //     &ExecuteMsg::RefuseOffer {
            //         global_offer_id: 1.to_string(),
            //     },
            //     &[],
            // ).unwrap();
            // println!("{:#?}", good_refuse_offer);

            // bad refuse offer
            // let bad_refuse_offer = app
            //     .execute_contract(
            //         Addr::unchecked("not-owner"),
            //         nft_loan_addr.clone(),
            //         &ExecuteMsg::RefuseOffer {
            //             global_offer_id: 1.to_string(),
            //         },
            //         &[],
            //     )
            //     .unwrap_err();
            // println!("{:#?}", bad_refuse_offer);

            // withdraw refused offer
            // let good_withdraw_refused_offer = app
            // .execute_contract(
            //     Addr::unchecked(OFFERER_ADDR.to_string()),
            //     nft_loan_addr.clone(),
            //     &ExecuteMsg::WithdrawRefusedOffer {
            //         global_offer_id: 1.to_string()
            //     },
            //     &[]
            // ).unwrap();
            // println!("{:#?}", good_withdraw_refused_offer);

            // withdraw refused offer
            // let bad_withdraw_refused_offer = app
            // .execute_contract(
            //     Addr::unchecked("not-offerer".to_string()),
            //     nft_loan_addr.clone(),
            //     &ExecuteMsg::WithdrawRefusedOffer {
            //         global_offer_id: 1.to_string()
            //     },
            //     &[]
            // ).unwrap_err();
            // println!("{:#?}", bad_withdraw_refused_offer);

            // repay borrowed funds
            let good_repay_borrowed_funds = app
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
            println!("{:#?}", good_repay_borrowed_funds);

            // withdraw defaulted loans

            // set owner

            // set fee distributor

            // set fee rate
        }
    }
}
