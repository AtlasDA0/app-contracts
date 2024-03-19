#[cfg(test)]
mod tests {
    use anyhow::bail;
    use cosmwasm_std::{Addr, BlockInfo, Coin, StdResult, Uint128};
    use cw_multi_test::Executor;
    use sg_multi_test::StargazeApp;
    use sg_std::NATIVE_DENOM;

    use nft_loans_nc::{
        msg::{ExecuteMsg, ExtensionResponse, QueryMsg},
        state::{CollateralInfo, LoanState, LoanTerms},
    };
    use utils::state::{AssetInfo, Sg721Token};

    use crate::{
        common_setup::{
            setup_accounts_and_block::setup_accounts,
            setup_loan::{configure_loan_assets, proper_loan_instantiate},
            setup_minter::common::constants::OWNER_ADDR,
        },
        loan::setup::{execute_msg::create_loan_function, test_msgs::CreateLoanParams},
    };

    const OFFERER_ADDR: &str = "offerer";
    const SG721_CONTRACT: &str = "contract3";
    const LOAN_DURATION: u64 = 15;
    const ADDITIONAL_DURATION: u64 = 76;
    const LOAN_AMOUNT: u128 = 100;
    const LOAN_INTEREST: u128 = 50;
    const ADDITIONAL_INTEREST: u128 = 67;

    fn setup_loan() -> (StargazeApp, Addr) {
        // setup test environment
        let (mut app, loan_addr, factory_addr) = proper_loan_instantiate();
        let (owner_address, _, _) = setup_accounts(&mut app);
        configure_loan_assets(&mut app, owner_address.clone(), factory_addr);
        let create_loan_params: CreateLoanParams<'_> = CreateLoanParams {
            app: &mut app,
            loan_contract_addr: loan_addr.clone(),
            owner_addr: owner_address.clone(),
        };
        create_loan_function(create_loan_params).unwrap();

        let _list_collaterals = app
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
                    terms: None,
                    comment: Some("be water, my friend".to_string()),
                    loan_preview: None,
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(25u128),
                }],
            )
            .unwrap();

        let __make_offer = app
            .execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::MakeOffer {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    terms: LoanTerms {
                        principle: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(LOAN_AMOUNT),
                        },
                        interest: Uint128::new(LOAN_INTEREST),
                        duration_in_blocks: LOAN_DURATION,
                    },
                    comment: Some("Obey the principles without being bound by them".to_string()),
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100),
                }],
            )
            .unwrap();
        // good accept offer
        let _accept_offer = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                loan_addr.clone(),
                &ExecuteMsg::AcceptOffer {
                    global_offer_id: 1.to_string(),
                },
                &[],
            )
            .unwrap();

        (app, loan_addr)
    }

    fn repay(app: &mut StargazeApp, loan_addr: Addr, amount: u128) -> anyhow::Result<()> {
        // repay borrowed funds
        let _good_repay_borrowed_funds = app.execute_contract(
            Addr::unchecked(OWNER_ADDR.to_string()),
            loan_addr.clone(),
            &ExecuteMsg::RepayBorrowedFunds { loan_id: 0 },
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(amount),
            }],
        )?;

        let res: CollateralInfo = app.wrap().query_wasm_smart(
            loan_addr.clone(),
            &QueryMsg::CollateralInfo {
                borrower: Addr::unchecked(OWNER_ADDR).to_string(),
                loan_id: 0,
            },
        )?;
        if res.state != LoanState::Ended {
            bail!("Expected loan state to be ended, got {:?}", res.state);
        }
        Ok(())
    }

    fn get_extension(app: &StargazeApp, loan_addr: Addr) -> StdResult<ExtensionResponse> {
        app.wrap().query_wasm_smart(
            loan_addr,
            &QueryMsg::Extension {
                borrower: OWNER_ADDR.to_string(),
                loan_id: 0,
            },
        )
    }

    fn wait_blocks(app: &mut StargazeApp, blocks: u64) {
        let block = app.block_info();
        app.set_block(BlockInfo {
            height: block.height + blocks,
            time: block.time,
            chain_id: block.chain_id.clone(),
        });
    }

    fn default_with_blocks(
        app: &mut StargazeApp,
        loan_addr: Addr,
        borrower: Addr,
        blocks: u64,
    ) -> anyhow::Result<()> {
        wait_blocks(app, blocks);

        let _withdraw_defaulted_loan = app.execute_contract(
            Addr::unchecked(OFFERER_ADDR.to_string()),
            loan_addr.clone(),
            &ExecuteMsg::WithdrawDefaultedLoan {
                borrower: borrower.to_string(),
                loan_id: 0,
            },
            &[],
        )?;

        let res: CollateralInfo = app.wrap().query_wasm_smart(
            loan_addr.clone(),
            &QueryMsg::CollateralInfo {
                borrower: Addr::unchecked(OWNER_ADDR).to_string(),
                loan_id: 0,
            },
        )?;
        if res.state != LoanState::Defaulted {
            bail!("Expected loan state to be defaulted, got {:?}", res.state);
        }

        Ok(())
    }

    #[test]
    fn ask_for_extension_and_repay() {
        // setup test environment
        let (mut app, loan_addr) = setup_loan();

        // Ask for a loan extension
        app.execute_contract(
            Addr::unchecked(OWNER_ADDR.to_string()),
            loan_addr.clone(),
            &ExecuteMsg::RequestExtension {
                loan_id: 0,
                comment: None,
                additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                additional_duration: ADDITIONAL_DURATION,
            },
            &[],
        )
        .unwrap();
        repay(&mut app, loan_addr, LOAN_AMOUNT + LOAN_INTEREST).unwrap();
    }

    #[test]
    fn ask_for_extension_and_fail_repay() {
        // setup test environment
        let (mut app, loan_addr) = setup_loan();

        // Ask for a loan extension
        app.execute_contract(
            Addr::unchecked(OWNER_ADDR.to_string()),
            loan_addr.clone(),
            &ExecuteMsg::RequestExtension {
                loan_id: 0,
                comment: None,
                additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                additional_duration: ADDITIONAL_DURATION,
            },
            &[],
        )
        .unwrap();
        wait_blocks(&mut app, LOAN_DURATION + 1);

        repay(
            &mut app,
            loan_addr,
            LOAN_AMOUNT + LOAN_INTEREST + ADDITIONAL_INTEREST,
        )
        .unwrap_err();
    }

    #[test]
    fn ask_for_extension_and_default() {
        // setup test environment
        let (mut app, loan_addr) = setup_loan();

        // Ask for a loan extension
        app.execute_contract(
            Addr::unchecked(OWNER_ADDR.to_string()),
            loan_addr.clone(),
            &ExecuteMsg::RequestExtension {
                loan_id: 0,
                comment: None,
                additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                additional_duration: ADDITIONAL_DURATION,
            },
            &[],
        )
        .unwrap();
        default_with_blocks(
            &mut app,
            loan_addr,
            Addr::unchecked(OWNER_ADDR.to_string()),
            LOAN_DURATION + 1,
        )
        .unwrap();
    }

    #[test]
    fn ask_for_extension_accept_and_repay() {
        // setup test environment
        let (mut app, loan_addr) = setup_loan();

        // Ask for a loan extension
        app.execute_contract(
            Addr::unchecked(OWNER_ADDR.to_string()),
            loan_addr.clone(),
            &ExecuteMsg::RequestExtension {
                loan_id: 0,
                comment: None,
                additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                additional_duration: ADDITIONAL_DURATION,
            },
            &[],
        )
        .unwrap();

        // Accept the loan extension
        app.execute_contract(
            Addr::unchecked(OFFERER_ADDR.to_string()),
            loan_addr.clone(),
            &ExecuteMsg::AcceptExtension {
                borrower: OWNER_ADDR.to_string(),
                loan_id: 0,
                extension_id: 0,
            },
            &[],
        )
        .unwrap();
        wait_blocks(&mut app, LOAN_DURATION + 1);
        repay(
            &mut app,
            loan_addr,
            LOAN_AMOUNT + LOAN_INTEREST + ADDITIONAL_INTEREST,
        )
        .unwrap();
    }

    #[test]
    fn ask_for_extension_accept_and_default() {
        // setup test environment
        let (mut app, loan_addr) = setup_loan();

        // Ask for a loan extension
        app.execute_contract(
            Addr::unchecked(OWNER_ADDR.to_string()),
            loan_addr.clone(),
            &ExecuteMsg::RequestExtension {
                loan_id: 0,
                comment: None,
                additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                additional_duration: ADDITIONAL_DURATION,
            },
            &[],
        )
        .unwrap();

        // Accept the loan extension
        app.execute_contract(
            Addr::unchecked(OFFERER_ADDR.to_string()),
            loan_addr.clone(),
            &ExecuteMsg::AcceptExtension {
                borrower: OWNER_ADDR.to_string(),
                loan_id: 0,
                extension_id: 0,
            },
            &[],
        )
        .unwrap();
        default_with_blocks(
            &mut app,
            loan_addr.clone(),
            Addr::unchecked(OWNER_ADDR.to_string()),
            LOAN_DURATION + 1,
        )
        .unwrap_err();
        default_with_blocks(
            &mut app,
            loan_addr,
            Addr::unchecked(OWNER_ADDR.to_string()),
            ADDITIONAL_DURATION,
        )
        .unwrap();
    }

    pub mod multiple_extensions {
        use super::*;

        #[test]
        fn ask_for_2_extensions() {
            // setup test environment
            let (mut app, loan_addr) = setup_loan();
            assert_eq!(
                get_extension(&app, loan_addr.clone()).unwrap().extension,
                None
            );
            // Ask for a loan extension
            app.execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::RequestExtension {
                    loan_id: 0,
                    comment: None,
                    additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                    additional_duration: ADDITIONAL_DURATION,
                },
                &[],
            )
            .unwrap();
            assert_eq!(
                get_extension(&app, loan_addr.clone())
                    .unwrap()
                    .extension
                    .unwrap()
                    .extension_id,
                0
            );
            // Ask for a loan extension
            app.execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::RequestExtension {
                    loan_id: 0,
                    comment: None,
                    additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                    additional_duration: ADDITIONAL_DURATION,
                },
                &[],
            )
            .unwrap();
            assert_eq!(
                get_extension(&app, loan_addr)
                    .unwrap()
                    .extension
                    .unwrap()
                    .extension_id,
                1
            );
        }
        #[test]
        fn cant_accept_old_extension() {
            // setup test environment
            let (mut app, loan_addr) = setup_loan();
            // Ask for a loan extension
            app.execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::RequestExtension {
                    loan_id: 0,
                    comment: None,
                    additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                    additional_duration: ADDITIONAL_DURATION,
                },
                &[],
            )
            .unwrap();
            // Ask for a loan extension
            app.execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::RequestExtension {
                    loan_id: 0,
                    comment: None,
                    additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                    additional_duration: ADDITIONAL_DURATION,
                },
                &[],
            )
            .unwrap();
            app.execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::AcceptExtension {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    extension_id: 0,
                },
                &[],
            )
            .unwrap_err();
        }

        #[test]
        fn accept_second_extension_repay() {
            // setup test environment
            let (mut app, loan_addr) = setup_loan();
            // Ask for a loan extension
            app.execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::RequestExtension {
                    loan_id: 0,
                    comment: None,
                    additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                    additional_duration: ADDITIONAL_DURATION,
                },
                &[],
            )
            .unwrap();
            // Ask for a loan extension
            app.execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::RequestExtension {
                    loan_id: 0,
                    comment: None,
                    additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                    additional_duration: ADDITIONAL_DURATION,
                },
                &[],
            )
            .unwrap();
            app.execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::AcceptExtension {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    extension_id: 1,
                },
                &[],
            )
            .unwrap();
            wait_blocks(&mut app, LOAN_DURATION + 1);
            repay(
                &mut app,
                loan_addr,
                LOAN_AMOUNT + LOAN_INTEREST + ADDITIONAL_INTEREST,
            )
            .unwrap();
        }
        #[test]
        fn accept_second_extension_default() {
            // setup test environment
            let (mut app, loan_addr) = setup_loan();
            // Ask for a loan extension
            app.execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::RequestExtension {
                    loan_id: 0,
                    comment: None,
                    additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                    additional_duration: ADDITIONAL_DURATION,
                },
                &[],
            )
            .unwrap();
            // Ask for a loan extension
            app.execute_contract(
                Addr::unchecked(OWNER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::RequestExtension {
                    loan_id: 0,
                    comment: None,
                    additional_interest: Uint128::from(ADDITIONAL_INTEREST),
                    additional_duration: ADDITIONAL_DURATION,
                },
                &[],
            )
            .unwrap();
            app.execute_contract(
                Addr::unchecked(OFFERER_ADDR.to_string()),
                loan_addr.clone(),
                &ExecuteMsg::AcceptExtension {
                    borrower: OWNER_ADDR.to_string(),
                    loan_id: 0,
                    extension_id: 1,
                },
                &[],
            )
            .unwrap();
            default_with_blocks(
                &mut app,
                loan_addr.clone(),
                Addr::unchecked(OWNER_ADDR.to_string()),
                LOAN_DURATION + 1,
            )
            .unwrap_err();
            default_with_blocks(
                &mut app,
                loan_addr,
                Addr::unchecked(OWNER_ADDR.to_string()),
                ADDITIONAL_DURATION,
            )
            .unwrap();
        }
    }
}
