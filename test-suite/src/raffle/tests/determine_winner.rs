#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, HexBinary, Uint128};
    use cw_multi_test::Executor;
    use nois::NoisCallback;
    use raffles::{
        error::ContractError,
        msg::{ExecuteMsg as RaffleExecuteMsg, QueryMsg as RaffleQueryMsg, RaffleResponse},
        state::RaffleState,
    };
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    use crate::{
        common_setup::{
            helpers::{assert_error, setup_block_time},
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_minter::common::constants::{NOIS_PROXY_ADDR, SG721_CONTRACT},
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_setup},
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };

    #[test]
    fn test_zero_tickets() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);
        // create raffle
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

        // skip purchasing tickets

        // try to determine winner before raffle ends
        let claim_but_no_randomness_yet = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &RaffleExecuteMsg::DetermineWinner { raffle_id: 0 },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(claim_but_no_randomness_yet),
            ContractError::WrongStateForClaim {
                status: RaffleState::Started,
            }
            .to_string(),
        );

        // move forward in time
        let current_time = app.block_info().time.clone();
        let current_block = app.block_info().height.clone();
        let chainid = app.block_info().chain_id.clone();

        setup_block_time(
            &mut app,
            current_time.clone().plus_seconds(130).nanos(),
            Some(current_block.clone() + 100),
            &chainid.clone(),
        );

        // try to determine winner before randomness exists in state
        let claim_but_no_randomness_yet = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &RaffleExecuteMsg::DetermineWinner { raffle_id: 0 },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(claim_but_no_randomness_yet),
            ContractError::WrongStateForClaim {
                status: RaffleState::Closed,
            }
            .to_string(),
        );

        // ensure only nois_proxy provides randomness
        let bad_recieve_randomness = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &RaffleExecuteMsg::NoisReceive {
                    callback: NoisCallback {
                        job_id: "raffle-0".to_string(),
                        published: current_time.clone(),
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
            Err(bad_recieve_randomness),
            ContractError::UnauthorizedReceive.to_string(),
        );
        // simulates the response from nois_proxy
        let _good_receive_randomness = app
            .execute_contract(
                Addr::unchecked(NOIS_PROXY_ADDR),
                raffle_addr.clone(),
                &RaffleExecuteMsg::NoisReceive {
                    callback: NoisCallback {
                        job_id: "raffle-0".to_string(),
                        published: current_time.clone(),
                        randomness: HexBinary::from_hex(
                            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa115",
                        )
                        .unwrap(),
                    },
                },
                &[],
            )
            .unwrap();

        // queries the raffle
        let res: RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_addr.clone(),
                &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();
        // verify randomness state has been updated
        assert!(
            res.raffle_info.unwrap().randomness.is_some(),
            "randomness should have been updated into the raffle state"
        );

        let _good_determine_winner = app
            .execute_contract(
                owner_addr.clone(),
                raffle_addr.clone(),
                &RaffleExecuteMsg::DetermineWinner { raffle_id: 0 },
                &[],
            )
            .unwrap();

        // queries the raffle
        let res: RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_addr.clone(),
                &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();

        // verify winner is always owner
        assert_eq!(
            res.raffle_info.clone().unwrap().owner,
            res.raffle_info.unwrap().winner.unwrap(),
            "winner should always be the owner if no tickets were bought"
        );
        // verify no tickets can be bought after raffle ends
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
        assert!(
            purchase_tickets.is_err(),
            "There should be an issue with purchasing a ticket once the winner is determined"
        );
    }

    #[test]
    fn test_multiple_tickets() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);
        // create raffle
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

        // Purchasing tickets for 3 people
        // ensure error if max tickets per address set is reached
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![three.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();

        // try to determine winner before raffle ends
        let claim_but_no_randomness_yet = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &RaffleExecuteMsg::DetermineWinner { raffle_id: 0 },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(claim_but_no_randomness_yet),
            ContractError::WrongStateForClaim {
                status: RaffleState::Started,
            }
            .to_string(),
        );

        // move forward in time
        let current_time = app.block_info().time;
        let current_block = app.block_info().height;
        let chainid = app.block_info().chain_id.clone();

        setup_block_time(
            &mut app,
            current_time.clone().plus_seconds(130).nanos(),
            Some(current_block.clone() + 100),
            &chainid.clone(),
        );

        // try to determine winner before randomness exists in state
        let claim_but_no_randomness_yet = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &RaffleExecuteMsg::DetermineWinner { raffle_id: 0 },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(claim_but_no_randomness_yet),
            ContractError::WrongStateForClaim {
                status: RaffleState::Closed,
            }
            .to_string(),
        );

        // simulates the response from nois_proxy
        let _receive_randomness = app
            .execute_contract(
                Addr::unchecked(NOIS_PROXY_ADDR),
                raffle_addr.clone(),
                &RaffleExecuteMsg::NoisReceive {
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
            .unwrap();

        // queries the raffle
        let res: RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_addr.clone(),
                &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();
        // verify randomness state has been updated
        assert!(
            res.raffle_info.unwrap().randomness.is_some(),
            "randomness should have been updated into the raffle state"
        );

        let owner_balance_before = app.wrap().query_balance(&owner_addr, "ustars").unwrap();
        let _good_determine_winner = app
            .execute_contract(
                owner_addr.clone(),
                raffle_addr.clone(),
                &RaffleExecuteMsg::DetermineWinner { raffle_id: 0 },
                &[],
            )
            .unwrap();

        // queries the raffle
        let res: RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_addr.clone(),
                &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();

        // verify winner is always owner
        assert_eq!(
            two,
            res.raffle_info.unwrap().winner.unwrap(),
            "winner should always be the owner if no tickets were bought"
        );
        // verify no tickets can be bought after raffle ends
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
        assert!(
            purchase_tickets.is_err(),
            "There should be an issue with purchasing a ticket once the winner is determined"
        );

        // We make sure the owner has more balance
        let owner_balance_after = app.wrap().query_balance(&owner_addr, "ustars").unwrap();

        assert_eq!(
            owner_balance_before.amount + Uint128::from(6u128), // 50% fee
            owner_balance_after.amount
        );
    }
}
