#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Timestamp, Uint128};
    use cw_multi_test::Executor;
    use raffles::state::{RaffleInfo, RaffleOptions};
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    #[cfg(feature = "sg")]
    use raffles::{
        error::ContractError,
        msg::{ExecuteMsg, QueryMsg as RaffleQueryMsg, RaffleResponse},
        state::{RaffleOptionsMsg, RaffleState},
    };

    use crate::{
        common_setup::{
            helpers::assert_error,
            setup_accounts_and_block::setup_accounts,
            setup_minter::common::constants::{FACTORY_ADDR, SG721_CONTRACT},
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{
            execute_msg::{create_raffle_function, create_raffle_setup},
            test_msgs::CreateRaffleParams,
        },
    };

    #[test]
    fn good_create_raffle() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
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
        create_raffle_setup(params);

        let res: RaffleResponse = app
            .wrap()
            .query_wasm_smart(raffle_addr, &RaffleQueryMsg::RaffleInfo { raffle_id: 0 })
            .unwrap();

        assert_eq!(
            res,
            RaffleResponse {
                raffle_id: 0,
                raffle_state: RaffleState::Started,
                raffle_info: Some(RaffleInfo {
                    owner: owner_addr,
                    assets: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "63".to_string()
                    })],
                    raffle_ticket_price: AssetInfo::Coin(coin(4, NATIVE_DENOM)),
                    number_of_tickets: 0,
                    randomness: None,
                    winner: None,
                    is_cancelled: false,
                    raffle_options: RaffleOptions {
                        raffle_start_timestamp: Timestamp::from_nanos(1647032400000000000),
                        raffle_duration: 1,

                        comment: None,
                        max_ticket_number: None,
                        max_ticket_per_address: None,
                        raffle_preview: 0,
                        min_ticket_number: None,
                    }
                })
            }
        )
    }

    #[test]
    fn good_default_options() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
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
        create_raffle_setup(params);

        let res: RaffleResponse = app
            .wrap()
            .query_wasm_smart(raffle_addr, &RaffleQueryMsg::RaffleInfo { raffle_id: 0 })
            .unwrap();

        assert_eq!(
            res,
            RaffleResponse {
                raffle_id: 0,
                raffle_state: RaffleState::Started,
                raffle_info: Some(RaffleInfo {
                    owner: owner_addr,
                    assets: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "63".to_string()
                    })],
                    raffle_ticket_price: AssetInfo::Coin(coin(4, NATIVE_DENOM)),
                    number_of_tickets: 0,
                    randomness: None,
                    winner: None,
                    is_cancelled: false,
                    raffle_options: RaffleOptions {
                        raffle_start_timestamp: Timestamp::from_nanos(1647032400000000000),
                        raffle_duration: 1,

                        comment: None,
                        max_ticket_number: None,
                        max_ticket_per_address: None,
                        raffle_preview: 0,
                        min_ticket_number: None,
                    }
                })
            }
        )
    }
    #[test]
    fn bad_ticket_price() {
        let (mut app, raffle_addr, _factory_addr) = proper_raffle_instantiate();
        let (owner_address, _, _) = setup_accounts(&mut app);
        // configure raffle assets
        configure_raffle_assets(
            &mut app,
            owner_address.clone(),
            Addr::unchecked(FACTORY_ADDR),
            true,
        );
        // create a standard raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_address.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::from(0u64),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: None,
        };
        let msg = create_raffle_function(params);
        // confirm owner is set
        assert!(msg.is_err(), "There should be an error on this response");

        assert_error(
            Err(msg.unwrap_err()),
            ContractError::InvalidTicketCost {}.to_string(),
        )
    }

    #[test]
    fn bad_creation_fee() {
        let (mut app, raffle_addr, _factory_addr) = proper_raffle_instantiate();
        let (owner_address, _, _) = setup_accounts(&mut app);
        // configure raffle assets
        configure_raffle_assets(
            &mut app,
            owner_address.clone(),
            Addr::unchecked(FACTORY_ADDR),
            true,
        );
        // create a standard raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_address.clone(),
            creation_fee: vec![],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: None,
        };
        let msg = create_raffle_function(params);
        // confirm owner is set
        assert!(msg.is_err(), "There should be an error on this response");

        assert_error(
            Err(msg.unwrap_err()),
            ContractError::InvalidRaffleFee {}.to_string(),
        )
    }

    #[test]
    fn bad_create_raffle_not_owner() {
        let (mut app, raffle_addr, _) = proper_raffle_instantiate();
        let (owner_address, one, _) = setup_accounts(&mut app);
        // configure raffle assets
        configure_raffle_assets(
            &mut app,
            owner_address.clone(),
            Addr::unchecked(FACTORY_ADDR),
            true,
        );
        // create a standard raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: one.clone(),
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
        let msg = create_raffle_function(params);
        // confirm owner is set
        assert!(msg.is_err(), "There should be an error on this response");

        assert_error(
            Err(msg.unwrap_err()),
            "Generic error: message sender is not owner of tokens being raffled".to_string(),
        )
    }

    #[test]
    fn bad_cancel_unauthorized() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr,
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
        create_raffle_setup(params);

        let invalid_cancel_raffle = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                raffle_addr.clone(),
                &ExecuteMsg::CancelRaffle { raffle_id: 0 },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(invalid_cancel_raffle),
            ContractError::Unauthorized {}.to_string(),
        );
    }

    #[test]
    fn bad_modify_raffle_unauthorized() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_address, _, _) = setup_accounts(&mut app);
        configure_raffle_assets(&mut app, owner_address.clone(), factory_addr, true);
        let params = CreateRaffleParams {
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
        create_raffle_function(params).unwrap();

        let invalid_modify_raffle = app
            .execute_contract(
                Addr::unchecked("not-admin"),
                raffle_addr.clone(),
                &ExecuteMsg::ModifyRaffle {
                    raffle_id: 0,
                    raffle_ticket_price: None,
                    raffle_options: RaffleOptionsMsg {
                        raffle_start_timestamp: None,
                        raffle_duration: None,

                        comment: Some("rust is dooope".to_string()),
                        max_ticket_number: None,
                        max_ticket_per_address: None,
                        raffle_preview: None,
                        min_ticket_number: None,
                    },
                },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(invalid_modify_raffle),
            ContractError::Unauthorized {}.to_string(),
        );
        // raffle not exists
        let invalid_modify_raffle = app.execute_contract(
            owner_address.clone(),
            raffle_addr.clone(),
            &ExecuteMsg::ModifyRaffle {
                raffle_id: 1,
                raffle_ticket_price: None,
                raffle_options: RaffleOptionsMsg {
                    raffle_start_timestamp: None,
                    raffle_duration: None,

                    comment: Some("rust is dooope".to_string()),
                    max_ticket_number: None,
                    max_ticket_per_address: None,
                    raffle_preview: None,
                    min_ticket_number: None,
                },
            },
            &[],
        );
        assert!(invalid_modify_raffle.is_err());
    }
    #[test]
    fn good_modify_raffle() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_address, _, _) = setup_accounts(&mut app);
        configure_raffle_assets(&mut app, owner_address.clone(), factory_addr, true);
        let params = CreateRaffleParams {
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
        create_raffle_function(params).unwrap();

        let _good_modify_raffle = app
            .execute_contract(
                owner_address.clone(),
                raffle_addr.clone(),
                &ExecuteMsg::ModifyRaffle {
                    raffle_id: 0,
                    raffle_ticket_price: None,
                    raffle_options: RaffleOptionsMsg {
                        raffle_start_timestamp: None,
                        raffle_duration: None,

                        comment: Some("rust is dooope".to_string()),
                        max_ticket_number: None,
                        max_ticket_per_address: None,
                        raffle_preview: None,
                        min_ticket_number: None,
                    },
                },
                &[],
            )
            .unwrap();

        let res: RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_addr.clone(),
                &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();
        // println!("{:#?}", res);

        assert_eq!(
            res,
            RaffleResponse {
                raffle_id: 0,
                raffle_state: RaffleState::Started,
                raffle_info: Some(RaffleInfo {
                    owner: owner_address.clone(),
                    assets: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: 63.to_string(),
                    })],
                    raffle_ticket_price: AssetInfo::Coin(Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(4),
                    }),
                    number_of_tickets: 0,
                    randomness: None,
                    winner: None,
                    is_cancelled: false,
                    raffle_options: RaffleOptions {
                        raffle_start_timestamp: Timestamp::from_nanos(1647032400000000000),
                        raffle_duration: 1u64,
                        comment: Some("rust is dooope".to_string()),
                        max_ticket_number: None,
                        max_ticket_per_address: None,
                        raffle_preview: 0,
                        min_ticket_number: None,
                    }
                })
            }
        );

        // new raffle lifecycle ends before current time
        let _invalid_modify_raffle = app
            .execute_contract(
                owner_address.clone(),
                raffle_addr.clone(),
                &ExecuteMsg::ModifyRaffle {
                    raffle_id: 0,
                    raffle_ticket_price: None,
                    raffle_options: RaffleOptionsMsg {
                        raffle_start_timestamp: Some(Timestamp::from_nanos(1647032399999999990)), // checks new raffle start time is < original
                        raffle_duration: Some(2),

                        comment: Some("rust is dooope".to_string()),
                        max_ticket_number: None,
                        max_ticket_per_address: None,
                        raffle_preview: None,
                        min_ticket_number: None,
                    },
                },
                &[],
            )
            .unwrap();

        let res: RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_addr.clone(),
                &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();
        println!("{:#?}", res);

        assert_eq!(
            res,
            RaffleResponse {
                raffle_id: 0,
                raffle_state: RaffleState::Started,
                raffle_info: Some(RaffleInfo {
                    owner: owner_address,
                    assets: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: 63.to_string(),
                    })],
                    raffle_ticket_price: AssetInfo::Coin(Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(4),
                    }),
                    number_of_tickets: 0,
                    randomness: None,
                    winner: None,
                    is_cancelled: false,
                    raffle_options: RaffleOptions {
                        raffle_start_timestamp: Timestamp::from_nanos(1647032400000000000),
                        raffle_duration: 2u64,
                        comment: Some("rust is dooope".to_string()),
                        max_ticket_number: None,
                        max_ticket_per_address: None,
                        raffle_preview: 0,
                        min_ticket_number: None,
                    }
                })
            }
        );
    }
}
