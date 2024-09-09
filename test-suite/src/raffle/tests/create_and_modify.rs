#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Timestamp, Uint128};
    use cw_multi_test::Executor;
    use raffles::state::{RaffleInfo, RaffleOptions, MAX_TICKET_NUMBER};
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    #[cfg(feature = "sg")]
    use raffles::{
        error::ContractError,
        msg::{ExecuteMsg, RaffleResponse},
        state::{RaffleOptionsMsg, RaffleState},
    };

    use crate::{
        common_setup::{
            app::StargazeApp, helpers::assert_error, msg::RaffleContracts,
            setup_accounts_and_block::setup_accounts, setup_raffle::proper_raffle_instantiate,
        },
        raffle::setup::{
            execute_msg::{create_raffle_function, create_raffle_setup},
            helpers::{mint_one_token, raffle_info, TokenMint},
            test_msgs::CreateRaffleParams,
        },
    };
    fn create_simple_raffle(
        app: &mut StargazeApp,
        contracts: &RaffleContracts,
        token: &TokenMint,
        owner_addr: Addr,
    ) -> anyhow::Result<()> {
        let params = CreateRaffleParams {
            app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr,
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id.to_string(),
            })],
            duration: None,
            gating: vec![],
            min_ticket_number: None,
            max_tickets: None,
        };
        create_raffle_setup(params)?;
        Ok(())
    }

    #[test]
    fn good_create_raffle() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        create_simple_raffle(&mut app, &contracts, &token, owner_addr.clone()).unwrap();

        let res = raffle_info(&app, &contracts, 0);

        assert_eq!(
            res,
            RaffleResponse {
                raffle_id: 0,
                raffle_state: RaffleState::Started,
                raffle_info: Some(RaffleInfo {
                    owner: owner_addr,
                    assets: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: token.nft.to_string(),
                        token_id: token.token_id
                    })],
                    raffle_ticket_price: AssetInfo::Coin(coin(4, NATIVE_DENOM)),
                    number_of_tickets: 0,
                    randomness: None,
                    winners: vec![],
                    is_cancelled: false,
                    raffle_options: RaffleOptions {
                        raffle_start_timestamp: Timestamp::from_nanos(1647032400000000000),
                        raffle_duration: 1,

                        comment: None,
                        max_ticket_number: Some(MAX_TICKET_NUMBER),
                        max_ticket_per_address: None,
                        raffle_preview: 0,
                        one_winner_per_asset: false,
                        whitelist: None,
                        gating_raffle: vec![],
                        min_ticket_number: None,
                    }
                })
            }
        )
    }

    #[test]
    fn good_default_options() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        create_simple_raffle(&mut app, &contracts, &token, owner_addr.clone()).unwrap();

        let res = raffle_info(&app, &contracts, 0);

        assert_eq!(
            res,
            RaffleResponse {
                raffle_id: 0,
                raffle_state: RaffleState::Started,
                raffle_info: Some(RaffleInfo {
                    owner: owner_addr,
                    assets: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: token.nft.to_string(),
                        token_id: token.token_id.clone()
                    })],
                    raffle_ticket_price: AssetInfo::Coin(coin(4, NATIVE_DENOM)),
                    number_of_tickets: 0,
                    randomness: None,
                    winners: vec![],
                    is_cancelled: false,
                    raffle_options: RaffleOptions {
                        raffle_start_timestamp: Timestamp::from_nanos(1647032400000000000),
                        raffle_duration: 1,

                        comment: None,
                        max_ticket_number: Some(MAX_TICKET_NUMBER),
                        max_ticket_per_address: None,
                        raffle_preview: 0,
                        one_winner_per_asset: false,
                        whitelist: None,
                        gating_raffle: vec![],
                        min_ticket_number: None,
                    }
                })
            }
        )
    }
    #[test]
    fn bad_ticket_price() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create a standard raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::from(0u64),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id.clone(),
            })],
            duration: None,
            gating: vec![],
            min_ticket_number: None,
            max_tickets: None,
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
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_address, _, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create a standard raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_address.clone(),
            creation_fee: vec![],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id,
            })],
            duration: None,
            gating: vec![],
            min_ticket_number: None,
            max_tickets: None,
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
        let (mut app, contracts) = proper_raffle_instantiate();
        let (_, one, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create a standard raffle
        let msg = create_simple_raffle(&mut app, &contracts, &token, one.clone());
        // confirm owner is set
        assert!(msg.is_err(), "There should be an error on this response");

        assert_error(
            Err(msg.unwrap_err()),
            "Generic error: message sender is not owner of tokens being raffled".to_string(),
        )
    }

    #[test]
    fn bad_cancel_unauthorized() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        create_simple_raffle(&mut app, &contracts, &token, owner_addr).unwrap();

        let invalid_cancel_raffle = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                contracts.raffle.clone(),
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
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        create_simple_raffle(&mut app, &contracts, &token, owner_addr.clone()).unwrap();

        let invalid_modify_raffle = app
            .execute_contract(
                Addr::unchecked("not-admin"),
                contracts.raffle.clone(),
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
                        one_winner_per_asset: false,
                        whitelist: None,
                        gating_raffle: vec![],
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
            owner_addr.clone(),
            contracts.raffle.clone(),
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
                    one_winner_per_asset: false,
                    whitelist: None,
                    gating_raffle: vec![],
                    min_ticket_number: None,
                },
            },
            &[],
        );
        assert!(invalid_modify_raffle.is_err());
    }
    #[test]
    fn good_modify_raffle() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        create_simple_raffle(&mut app, &contracts, &token, owner_addr.clone()).unwrap();

        let _good_modify_raffle = app
            .execute_contract(
                owner_addr.clone(),
                contracts.raffle.clone(),
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
                        one_winner_per_asset: false,
                        whitelist: None,
                        gating_raffle: vec![],
                        min_ticket_number: None,
                    },
                },
                &[],
            )
            .unwrap();

        let res = raffle_info(&app, &contracts, 0);

        assert_eq!(
            res,
            RaffleResponse {
                raffle_id: 0,
                raffle_state: RaffleState::Started,
                raffle_info: Some(RaffleInfo {
                    owner: owner_addr.clone(),
                    assets: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: token.nft.to_string(),
                        token_id: token.token_id.clone()
                    })],
                    raffle_ticket_price: AssetInfo::Coin(Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(4),
                    }),
                    number_of_tickets: 0,
                    randomness: None,
                    winners: vec![],
                    is_cancelled: false,
                    raffle_options: RaffleOptions {
                        raffle_start_timestamp: Timestamp::from_nanos(1647032400000000000),
                        raffle_duration: 1u64,
                        comment: Some("rust is dooope".to_string()),
                        max_ticket_number: Some(MAX_TICKET_NUMBER),
                        max_ticket_per_address: None,
                        raffle_preview: 0,
                        one_winner_per_asset: false,
                        whitelist: None,
                        gating_raffle: vec![],
                        min_ticket_number: None,
                    }
                })
            }
        );

        // new raffle lifecycle ends before current time
        let _invalid_modify_raffle = app
            .execute_contract(
                owner_addr.clone(),
                contracts.raffle.clone(),
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
                        one_winner_per_asset: false,
                        whitelist: None,
                        gating_raffle: vec![],
                        min_ticket_number: None,
                    },
                },
                &[],
            )
            .unwrap();

        let res = raffle_info(&app, &contracts, 0);

        assert_eq!(
            res,
            RaffleResponse {
                raffle_id: 0,
                raffle_state: RaffleState::Started,
                raffle_info: Some(RaffleInfo {
                    owner: owner_addr,
                    assets: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: token.nft.to_string(),
                        token_id: token.token_id.clone()
                    })],
                    raffle_ticket_price: AssetInfo::Coin(Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(4),
                    }),
                    number_of_tickets: 0,
                    randomness: None,
                    winners: vec![],
                    is_cancelled: false,
                    raffle_options: RaffleOptions {
                        raffle_start_timestamp: Timestamp::from_nanos(1647032400000000000),
                        raffle_duration: 2u64,
                        comment: Some("rust is dooope".to_string()),
                        max_ticket_number: Some(MAX_TICKET_NUMBER),
                        max_ticket_per_address: None,
                        raffle_preview: 0,
                        one_winner_per_asset: false,
                        whitelist: None,
                        gating_raffle: vec![],
                        min_ticket_number: None,
                    }
                })
            }
        );
    }
}
