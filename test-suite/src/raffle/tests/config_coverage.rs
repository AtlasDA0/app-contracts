#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Decimal, HexBinary, Uint128};
    use cw_multi_test::Executor;
    use nois::NoisCallback;
    use raffles::{
        error::ContractError,
        msg::{ExecuteMsg, QueryMsg as RaffleQueryMsg},
        state::{Config, StakerFeeDiscount},
    };
    use utils::state::{
        AssetInfo, Locks, Sg721Token, SudoMsg as RaffleSudoMsg, NATIVE_DENOM, NOIS_AMOUNT,
    };

    use crate::{
        common_setup::{
            contract_boxes::custom_mock_app,
            helpers::assert_error,
            setup_accounts_and_block::setup_accounts,
            setup_minter::common::constants::{
                CREATION_FEE_AMNT, MINT_PRICE, NOIS_PROXY_ADDR, OWNER_ADDR, RAFFLE_NAME,
                RAFFLE_TAX, SG721_CONTRACT,
            },
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{
            execute_msg::{
                buy_tickets_template, create_raffle_function, instantate_raffle_contract,
            },
            test_msgs::{CreateRaffleParams, InstantiateRaffleParams, PurchaseTicketsParams},
        },
    };

    #[test]
    fn test_raffle_config_query() {
        let mut app = custom_mock_app();
        let params = InstantiateRaffleParams {
            app: &mut app,
            admin_account: Addr::unchecked(OWNER_ADDR),
            funds_amount: MINT_PRICE,
            fee_rate: RAFFLE_TAX,
            name: RAFFLE_NAME.into(),
            nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
            nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
        };
        let raffle_addr = instantate_raffle_contract(params).unwrap();

        let query_config: Config = app
            .wrap()
            .query_wasm_smart(raffle_addr, &RaffleQueryMsg::Config {})
            .unwrap();
        assert_eq!(
            query_config,
            Config {
                name: RAFFLE_NAME.into(),
                owner: Addr::unchecked(OWNER_ADDR),
                fee_addr: Addr::unchecked(OWNER_ADDR),
                last_raffle_id: Some(0),
                minimum_raffle_duration: 1,
                minimum_raffle_timeout: 120,
                max_tickets_per_raffle: None,
                raffle_fee: RAFFLE_TAX,
                nois_proxy_addr: Addr::unchecked(NOIS_PROXY_ADDR),
                nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
                creation_coins: vec![coin(CREATION_FEE_AMNT, NATIVE_DENOM)],
                locks: Locks {
                    lock: false,
                    sudo_lock: false,
                },
                atlas_dao_nft_address: None,
                staker_fee_discount: StakerFeeDiscount {
                    discount: Decimal::zero(),
                    minimum_amount: Uint128::zero()
                }
            }
        )
    }

    #[test]
    fn test_raffle_contract_config_permissions_coverage() {
        let (mut app, raffle_addr, _) = proper_raffle_instantiate();
        let current_time = app.block_info().time;
        // errors
        // unable to update contract config
        let error_updating_config = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                raffle_addr.clone(),
                &ExecuteMsg::UpdateConfig {
                    name: Some("not-owner".to_string()),
                    owner: None,
                    fee_addr: None,
                    minimum_raffle_duration: None,
                    minimum_raffle_timeout: None,
                    raffle_fee: None,
                    nois_proxy_addr: None,
                    nois_proxy_coin: None,
                    creation_coins: None,
                    max_tickets_per_raffle: None,
                    atlas_dao_nft_address: None,
                    staker_fee_discount: None,
                },
                &[],
            )
            .unwrap_err();
        // unable to lock contract
        let error_locking_contract = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                raffle_addr.clone(),
                &raffles::msg::ExecuteMsg::ToggleLock { lock: true },
                &[],
            )
            .unwrap_err();
        // unable to provide randomness unless nois_proxy address is sending msg
        let error_not_proxy_providing_randomness = app
            .execute_contract(
                Addr::unchecked("not-nois-proxy"),
                raffle_addr.clone(),
                &raffles::msg::ExecuteMsg::NoisReceive {
                    callback: NoisCallback {
                        job_id: "raffle-0".to_string(),
                        published: current_time,
                        randomness: HexBinary::from_hex(
                            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa115",
                        )
                        .unwrap(),
                    },
                },
                &[],
            )
            .unwrap_err();

        assert_error(
            Err(error_updating_config),
            ContractError::Unauthorized {}.to_string(),
        );
        assert_error(
            Err(error_locking_contract),
            ContractError::Unauthorized {}.to_string(),
        );
        assert_error(
            Err(error_not_proxy_providing_randomness),
            ContractError::UnauthorizedReceive {}.to_string(),
        );
        let _updating_config = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                raffle_addr.clone(),
                &ExecuteMsg::UpdateConfig {
                    name: Some("new-owner".to_string()),
                    owner: Some("new-owner".to_string()),
                    fee_addr: Some("new-owner".to_string()),
                    minimum_raffle_duration: Some(60),
                    minimum_raffle_timeout: Some(240),
                    raffle_fee: Some(Decimal::percent(99)),
                    nois_proxy_addr: Some("new-owner".to_string()),
                    nois_proxy_coin: Some(coin(NOIS_AMOUNT, NATIVE_DENOM)),
                    creation_coins: Some(vec![coin(420, "new-new")]),
                    max_tickets_per_raffle: None,
                    atlas_dao_nft_address: None,
                    staker_fee_discount: None,
                },
                &[],
            )
            .unwrap();
        // good responses
        let res: Config = app
            .wrap()
            .query_wasm_smart(raffle_addr.clone(), &RaffleQueryMsg::Config {})
            .unwrap();
        println!("{:#?}", res);
        assert_eq!(
            res,
            Config {
                name: "new-owner".to_string(),
                owner: Addr::unchecked("new-owner"),
                fee_addr: Addr::unchecked("new-owner"),
                last_raffle_id: Some(0),
                minimum_raffle_duration: 60,
                minimum_raffle_timeout: 240,
                max_tickets_per_raffle: None,
                raffle_fee: Decimal::percent(99),
                nois_proxy_addr: Addr::unchecked("new-owner"),
                nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
                creation_coins: vec![coin(420, "new-new")],
                locks: Locks {
                    lock: false,
                    sudo_lock: false,
                },
                atlas_dao_nft_address: None,
                staker_fee_discount: StakerFeeDiscount {
                    discount: Decimal::zero(),
                    minimum_amount: Uint128::zero()
                }
            }
        )
    }

    #[test]
    fn good_toggle_lock() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_address, one, _) = setup_accounts(&mut app);
        configure_raffle_assets(&mut app, owner_address.clone(), factory_addr, true);
        let create_raffle_params: CreateRaffleParams<'_> = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_address.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: None,
        };
        create_raffle_function(create_raffle_params).unwrap();

        let _invalid_toggle_lock = app
            .execute_contract(
                owner_address.clone(),
                raffle_addr.clone(),
                &ExecuteMsg::ToggleLock { lock: true },
                &[],
            )
            .unwrap();
        // confirm the state is now true
        let res: Config = app
            .wrap()
            .query_wasm_smart(raffle_addr.to_string(), &RaffleQueryMsg::Config {})
            .unwrap();
        assert!(res.locks.lock);

        let create_raffle_params: CreateRaffleParams<'_> = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_address.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: None,
        };

        // confirm raffles cannot be made & tickets cannot be bought
        let locked_creation = create_raffle_function(create_raffle_params).unwrap_err();
        assert_error(
            Err(locked_creation),
            ContractError::ContractIsLocked {}.to_string(),
        );

        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let purchase_tickets = buy_tickets_template(params);
        assert!(purchase_tickets.is_ok());
    }

    #[test]
    fn good_toggle_sudo_lock() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_address, one, _) = setup_accounts(&mut app);
        configure_raffle_assets(&mut app, owner_address.clone(), factory_addr, true);
        let create_raffle_params: CreateRaffleParams<'_> = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_address.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: None,
        };
        create_raffle_function(create_raffle_params).unwrap();

        let _invalid_toggle_lock = app
            .wasm_sudo(
                raffle_addr.clone(),
                &RaffleSudoMsg::ToggleLock { lock: true },
            )
            .unwrap();

        // confirm the state is now true
        let res: Config = app
            .wrap()
            .query_wasm_smart(raffle_addr.to_string(), &RaffleQueryMsg::Config {})
            .unwrap();
        assert!(res.locks.sudo_lock);

        let create_raffle_params: CreateRaffleParams<'_> = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_address.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: None,
        };

        // confirm raffles cannot be made
        let locked_creation = create_raffle_function(create_raffle_params).unwrap_err();
        assert_error(
            Err(locked_creation),
            ContractError::ContractIsLocked {}.to_string(),
        );

        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let purchase_tickets = buy_tickets_template(params);
        assert!(purchase_tickets.is_ok());
    }
}
