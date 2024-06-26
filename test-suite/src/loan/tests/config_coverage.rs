#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Decimal};
    use cw_multi_test::Executor;
    use sg_std::NATIVE_DENOM;

    use nft_loans_nc::{
        error::ContractError,
        msg::{ExecuteMsg, QueryMsg as LoanQueryMsg},
        state::Config,
    };
    use utils::state::{Locks, SudoMsg as LoanSudoMsg};

    use crate::{
        common_setup::{
            contract_boxes::custom_mock_app,
            helpers::assert_error,
            setup_accounts_and_block::setup_accounts,
            setup_loan::{configure_loan_assets, proper_loan_instantiate},
            setup_minter::common::constants::{
                LOAN_INTEREST_TAX, LOAN_NAME, MINT_PRICE, MIN_COLLATERAL_LISTING, OWNER_ADDR,
                TREASURY_ADDR,
            },
        },
        loan::setup::{
            execute_msg::{create_loan_function, instantate_loan_contract},
            test_msgs::{CreateLoanParams, InstantiateParams},
        },
    };

    #[test]
    fn test_config_query() {
        let mut app = custom_mock_app();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: Decimal::percent(50),
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
                treasury_addr: Addr::unchecked(TREASURY_ADDR.to_string()),
                fee_rate: LOAN_INTEREST_TAX,
                global_offer_index: 0,
                global_collection_offer_index: 0,
                listing_fee_coins: vec![
                    Coin::new(MIN_COLLATERAL_LISTING, NATIVE_DENOM),
                    Coin::new(MIN_COLLATERAL_LISTING, "usstars"),
                ],
                locks: Locks {
                    lock: false,
                    sudo_lock: false,
                },
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
                    treasury_addr: TREASURY_ADDR.into(),
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
                treasury_addr: Addr::unchecked(TREASURY_ADDR.to_string()),
                fee_rate: Decimal::percent(10),
                listing_fee_coins: vec![
                    coin(4, "uflix"),
                    coin(5, "uscrt"),
                    coin(6, "uatom"),
                    coin(7, "ustars"),
                ],
                global_offer_index: 0,
                global_collection_offer_index: 0,
                locks: Locks {
                    lock: false,
                    sudo_lock: false,
                }
            }
        )
    }

    #[test]
    fn good_toggle_lock() {
        let (mut app, loan_addr, factory_addr) = proper_loan_instantiate();
        let (owner_address, _, _) = setup_accounts(&mut app);
        configure_loan_assets(&mut app, owner_address.clone(), factory_addr);
        let create_raffle_params: CreateLoanParams<'_> = CreateLoanParams {
            app: &mut app,
            loan_contract_addr: loan_addr.clone(),
            owner_addr: owner_address.clone(),
            // creation_fee: vec![coin(4, NATIVE_DENOM)],
            // ticket_price: None,
            // max_ticket_per_addr: None,
        };
        create_loan_function(create_raffle_params).unwrap();

        let _invalid_toggle_lock = app
            .execute_contract(
                owner_address.clone(),
                loan_addr.clone(),
                &ExecuteMsg::ToggleLock { lock: true },
                &[],
            )
            .unwrap();
        // confirm the state is now true
        let res: Config = app
            .wrap()
            .query_wasm_smart(loan_addr.to_string(), &LoanQueryMsg::Config {})
            .unwrap();
        assert!(res.locks.lock);

        let create_raffle_params: CreateLoanParams<'_> = CreateLoanParams {
            app: &mut app,
            loan_contract_addr: loan_addr.clone(),
            owner_addr: owner_address.clone(),
        };

        // confirm loans cannot be made,
        // loans can be cancelled & withdrawn,
        //
        let locked_creation = create_loan_function(create_raffle_params).unwrap_err();
        assert_error(
            Err(locked_creation),
            ContractError::ContractIsLocked {}.to_string(),
        );

        let _params = CreateLoanParams {
            app: &mut app,
            loan_contract_addr: loan_addr.clone(),
            owner_addr: owner_address.clone(),
        };
    }

    #[test]
    fn good_toggle_sudo_lock() {
        let (mut app, loan_addr, factory_addr) = proper_loan_instantiate();
        let (owner_address, _, _) = setup_accounts(&mut app);
        configure_loan_assets(&mut app, Addr::unchecked("owner").clone(), factory_addr);
        let create_loan_params: CreateLoanParams<'_> = CreateLoanParams {
            app: &mut app,
            loan_contract_addr: loan_addr.clone(),
            owner_addr: owner_address.clone(),
        };
        create_loan_function(create_loan_params).unwrap();

        let _invalid_toggle_lock = app
            .wasm_sudo(loan_addr.clone(), &LoanSudoMsg::ToggleLock { lock: true })
            .unwrap();

        // confirm the state is now true
        let res: Config = app
            .wrap()
            .query_wasm_smart(loan_addr.to_string(), &LoanQueryMsg::Config {})
            .unwrap();
        assert!(res.locks.sudo_lock);

        let create_loan_params: CreateLoanParams<'_> = CreateLoanParams {
            app: &mut app,
            loan_contract_addr: loan_addr.clone(),
            owner_addr: Addr::unchecked("owner"),
            // creation_fee: vec![coin(4, NATIVE_DENOM)],
            // ticket_price: None,
            // max_ticket_per_addr: None,
        };

        // confirm raffles cannot be made & tickets cannot be bought
        let locked_creation = create_loan_function(create_loan_params).unwrap_err();
        assert_error(
            Err(locked_creation),
            ContractError::ContractIsLocked {}.to_string(),
        );

        let params = CreateLoanParams {
            app: &mut app,
            loan_contract_addr: loan_addr.clone(),
            owner_addr: Addr::unchecked("owner"),
            // msg_senders: vec![one.clone()],
            // raffle_id: 0,
            // num_tickets: 1,
            // funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let deposit_collateral = create_loan_function(params).unwrap_err();

        assert_error(
            Err(deposit_collateral),
            ContractError::ContractIsLocked {}.to_string(),
        )
    }

    #[test]
    fn change_some_config() {
        let (mut app, loan_addr, _factory_addr) = proper_loan_instantiate();
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
            .query_wasm_smart(loan_addr.clone(), &LoanQueryMsg::Config {})
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
            .query_wasm_smart(loan_addr.clone(), &LoanQueryMsg::Config {})
            .unwrap();
        assert_eq!(res.owner, "new-admin".to_string());
    }
}
