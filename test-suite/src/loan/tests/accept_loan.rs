#[cfg(test)]
mod tests {
    use crate::common_setup::app::StargazeApp;
    use cosmwasm_std::{coin, Addr, BlockInfo, Coin, Decimal, Empty, Timestamp, Uint128};
    use cw_multi_test::{BankSudo, Executor, SudoMsg};
    use nft_loans_nc::{
        error::ContractError,
        msg::{ExecuteMsg, InstantiateMsg},
        state::{CollateralInfo, LoanState, LoanTerms},
    };
    use sg721::CollectionInfo;
    use sg_std::NATIVE_DENOM;
    use utils::state::{AssetInfo, Sg721Token};
    use vending_factory::{
        msg::VendingMinterCreateMsg,
        state::{ParamsExtension, VendingMinterParams},
    };

    use crate::common_setup::{
        contract_boxes::{
            contract_nft_loans, contract_sg721_base, contract_vending_factory,
            contract_vending_minter, custom_mock_app,
        },
        helpers::assert_error,
        setup_minter::common::constants::OWNER_ADDR,
    };

    const OFFERER_ADDR: &str = "offerer";
    const DEPOSITOR_ADDR: &str = "depositor";
    const BORROWER_ADDR: &str = "borrower";
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
                        coin(55, NATIVE_DENOM.to_string()),
                        coin(45, "usstars".to_string()),
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

    #[test]
    fn loan() {
        let (mut app, nft_loan_addr, factory_addr) = proper_instantiate();

        let current_time = app.block_info().time;

        // create nft minter
        let _create_nft_minter = app.execute_contract(
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

        // VENDING_MINTER is minter
        let _mint_nft_tokens = app
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
        let _mint_nft_tokens_again = app
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

        // token id 41
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
        // token id 56
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
        // println!("{:#?}", _grant_approval);

        // loan-id 0
        let _good_deposit_collateral = app
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
                    comment: Some("Real living is living for others".to_string()),
                    loan_preview: None,
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(55u128),
                }],
            )
            .unwrap();
        // println!("{:#?}", _good_deposit_collateral);

        // too_little_loan
        let funds_dont_match_terms = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::AcceptLoan {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    comment: Some("Real living is living for others".to_string()),
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(50),
                }],
            )
            .unwrap_err();
        // println!("{:#?}", funds_dont_match_terms);

        assert_error(
            Err(funds_dont_match_terms),
            ContractError::FundsDontMatchTerms {}.to_string(),
        );

        // accept loan
        let _good_accept_loan = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::AcceptLoan {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    comment: Some("Real living is living for others".to_string()),
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100),
                }],
            )
            .unwrap();
        let res: CollateralInfo = app
            .wrap()
            .query_wasm_smart(
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::QueryMsg::CollateralInfo {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                },
            )
            .unwrap();
        assert_eq!(res.state, LoanState::Started);

        // verify collateral cannot be withdraw after loan is accepted

        // move forward in time
        let current_time = app.block_info().time;
        let current_block = app.block_info().height;
        let chainid = app.block_info().chain_id.clone();

        println!("{:#?}", current_block);

        app.set_block(BlockInfo {
            height: current_block + 20,
            time: current_time.clone().plus_seconds(20),
            chain_id: chainid.clone(),
        });

        // verify defaulted loan
        let _bad_repay_loan = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                nft_loan_addr.clone(),
                &ExecuteMsg::RepayBorrowedFunds { loan_id: 0 },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(150),
                }],
            )
            .unwrap_err();
        // println!("{:#?}", bad_repay_loan);

        let _res: CollateralInfo = app
            .wrap()
            .query_wasm_smart(
                nft_loan_addr.clone(),
                &nft_loans_nc::msg::QueryMsg::CollateralInfo {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                },
            )
            .unwrap();
        // println!("{:#?}", res);
    }
}
