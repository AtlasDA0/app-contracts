#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, testing::mock_env, Addr, Decimal, HexBinary, Uint128};
    use cw_multi_test::Executor;
    use nois::NoisCallback;
    use raffles::{error::ContractError, msg::ExecuteMsg as RaffleExecuteMsg, state::RaffleState};
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    use crate::{
        common_setup::{
            helpers::{assert_error, setup_block_time},
            nois_proxy::RANDOMNESS_SEED,
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_raffle::{proper_raffle_instantiate, proper_raffle_instantiate_precise},
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_setup},
            helpers::{finish_raffle_timeout, mint_one_token, raffle_info},
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };

    #[test]
    fn test_zero_tickets() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id.clone(),
            })],
            duration: None,
            min_ticket_number: None,
            max_tickets: None,
        };
        create_raffle_setup(params).unwrap();

        // skip purchasing tickets

        // try to determine winner before raffle ends
        let err = app
            .execute_contract(
                contracts.nois.clone(),
                contracts.raffle.clone(),
                &RaffleExecuteMsg::NoisReceive {
                    callback: NoisCallback {
                        job_id: "raffle-0".to_string(),
                        published: mock_env().block.time,
                        randomness: HexBinary::from_hex(RANDOMNESS_SEED).unwrap(),
                    },
                },
                &[],
            )
            .unwrap_err();
        assert_error(
            Err(err),
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
            Some(current_block + 100),
            &chainid.clone(),
        );

        // ensure only nois_proxy provides randomness
        let bad_recieve_randomness = app
            .execute_contract(
                one.clone(),
                contracts.raffle.clone(),
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
            .unwrap_err();
        assert_error(
            Err(bad_recieve_randomness),
            ContractError::UnauthorizedReceive.to_string(),
        );
        // simulates the response from nois_proxy
        finish_raffle_timeout(&mut app, &contracts, 0, 0).unwrap();

        // queries the raffle
        let res = raffle_info(&app, &contracts, 0).raffle_info.unwrap();
        // verify randomness state has been updated
        assert!(
            res.randomness.is_some(),
            "randomness should have been updated into the raffle state"
        );

        // verify winner is always owner
        assert_eq!(
            res.owner, res.winners[0],
            "winner should always be the owner if no tickets were bought"
        );
        // verify no tickets can be bought after raffle ends
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
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
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id.to_string(),
            })],
            duration: None,
            min_ticket_number: None,
            max_tickets: None,
        };
        create_raffle_setup(params).unwrap();

        // Purchasing tickets for 3 people
        // ensure error if max tickets per address set is reached
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
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
                contracts.nois.clone(),
                contracts.raffle.clone(),
                &RaffleExecuteMsg::NoisReceive {
                    callback: NoisCallback {
                        job_id: "raffle-0".to_string(),
                        published: mock_env().block.time,
                        randomness: HexBinary::from_hex(RANDOMNESS_SEED).unwrap(),
                    },
                },
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
            Some(current_block + 100),
            &chainid.clone(),
        );

        let owner_balance_before = app.wrap().query_balance(&owner_addr, "ustars").unwrap();
        finish_raffle_timeout(&mut app, &contracts, 0, 0).unwrap();

        // queries the raffle
        let res = raffle_info(&app, &contracts, 0).raffle_info.unwrap();
        // verify randomness state has been updated
        assert!(
            res.randomness.is_some(),
            "randomness should have been updated into the raffle state"
        );

        // verify winner is always owner
        assert_eq!(
            res.owner, res.winners[0],
            "winner should always be the owner if no tickets were bought"
        );
        // verify no tickets can be bought after raffle ends
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
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

    #[test]
    fn close_after_all_tickets_sold() {
        let (mut app, contracts) = proper_raffle_instantiate_precise(Some(10));
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id.to_string(),
            })],
            duration: None,
            min_ticket_number: None,
            max_tickets: None,
        };
        create_raffle_setup(params).unwrap();

        // Purchasing tickets for 3 people
        // ensure error if max tickets per address set is reached
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 3,
            funds_send: vec![coin(12, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 0,
            num_tickets: 3,
            funds_send: vec![coin(12, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![three.clone()],
            raffle_id: 0,
            num_tickets: 4,
            funds_send: vec![coin(16, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();

        let res = raffle_info(&app, &contracts, 0);
        assert_eq!(res.raffle_info.unwrap().number_of_tickets, 10);

        // We can't buy anymore tickets after the last one is sold
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![three.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let purchase_err = buy_tickets_template(params).unwrap_err();
        assert_error(
            Err(purchase_err),
            ContractError::CantBuyTickets {}.to_string(),
        );

        let owner_balance_before = app.wrap().query_balance(&owner_addr, "ustars").unwrap();
        finish_raffle_timeout(&mut app, &contracts, 0, 1).unwrap();

        // queries the raffle
        let res = raffle_info(&app, &contracts, 0).raffle_info.unwrap();
        // verify randomness state has been updated
        assert!(
            res.randomness.is_some(),
            "randomness should have been updated into the raffle state"
        );

        // verify winner is always owner
        assert_eq!(
            two, res.winners[0],
            "winner should always be the owner if no tickets were bought"
        );
        // verify no tickets can be bought after raffle ends
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
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
            owner_balance_before.amount + Decimal::percent(50) * Uint128::from(4 * 10u128), // 50% fee of 10 tickets
            owner_balance_after.amount
        );
    }

    #[test]
    fn close_after_minimum_tickets_sold() {
        let (mut app, contracts) = proper_raffle_instantiate_precise(Some(10));
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id.to_string(),
            })],
            duration: None,
            min_ticket_number: Some(4),
            max_tickets: None,
        };
        create_raffle_setup(params).unwrap();

        // Purchasing tickets for 3 people
        // ensure error if max tickets per address set is reached
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 3,
            funds_send: vec![coin(12, "ustars")],
        };
        let _purchase_tickets = buy_tickets_template(params).unwrap();

        let one_balance_before = app.wrap().query_balance(&one, "ustars").unwrap().amount;
        let owner_balance_before = app.wrap().query_balance(&owner_addr, "ustars").unwrap();

        finish_raffle_timeout(&mut app, &contracts, 0, 130).unwrap();

        let one_balance_after = app.wrap().query_balance(&one, "ustars").unwrap().amount;

        // queries the raffle
        let res = raffle_info(&app, &contracts, 0).raffle_info.unwrap();
        // verify randomness state has been updated
        assert!(
            res.randomness.is_some(),
            "randomness should have been updated into the raffle state"
        );

        // verify winner is always owner
        assert_eq!(owner_addr, res.winners[0], "You have the wrong winner ");

        // verify no tickets can be bought after raffle ends
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
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
            owner_balance_before.amount, // Nothing happens, because the minimum was not reached
            owner_balance_after.amount
        );
        assert_eq!(
            one_balance_before + Uint128::from(4 * 3u128), // 100% fee of 3 tickets
            one_balance_after
        );
    }

    #[test]
    fn admin_randomness() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id.to_string(),
            })],
            duration: None,
            min_ticket_number: None,
            max_tickets: None,
        };
        create_raffle_setup(params).unwrap();

        // Purchasing tickets for 3 people
        // ensure error if max tickets per address set is reached
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 3,
            funds_send: vec![coin(12, "ustars")],
        };
        let _purchase_tickets = buy_tickets_template(params).unwrap();

        app.execute_contract(
            Addr::unchecked("bad-person"),
            contracts.raffle.clone(),
            &RaffleExecuteMsg::UpdateRandomness { raffle_id: 0 },
            &[],
        )
        .unwrap_err();

        app.execute_contract(
            owner_addr,
            contracts.raffle.clone(),
            &RaffleExecuteMsg::UpdateRandomness { raffle_id: 0 },
            &[],
        )
        .unwrap();

        finish_raffle_timeout(&mut app, &contracts, 0, 130).unwrap();
    }
}
