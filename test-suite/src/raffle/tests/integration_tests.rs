#[cfg(test)]
mod tests {

    use cosmwasm_std::{coin, Addr, BlockInfo, Uint128};
    use cw721::{ApprovalsResponse, OwnerOfResponse};
    #[cfg(feature = "sg")]
    use sg721_base::QueryMsg as Sg721QueryMsg;
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    use raffles::{
        msg::{ConfigResponse, QueryMsg as RaffleQueryMsg},
        state::RaffleState,
    };

    use crate::{
        common_setup::{
            app::StargazeApp,
            helpers::assert_treasury_balance,
            msg::RaffleContracts,
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_minter::common::constants::TREASURY_ADDR,
            setup_raffle::proper_raffle_instantiate,
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_setup},
            helpers::{
                finish_raffle_timeout, mint_additional_token, mint_one_token, raffle_info,
                TokenMint,
            },
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };
    fn create_simple_raffle(
        app: &mut StargazeApp,
        contracts: &RaffleContracts,
        token: &TokenMint,
        owner_addr: Addr,
    ) {
        let params = CreateRaffleParams {
            app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(10),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string().to_string(),
                token_id: token.token_id.clone(),
            })],
            duration: None,
            gating: vec![],
            min_ticket_number: None,
            max_tickets: None,
        };
        create_raffle_setup(params).unwrap();
    }

    #[test]
    fn two_raffle_participants() {
        // create testing app
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, one, two) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);

        create_simple_raffle(&mut app, &contracts, &token, owner_addr.clone());

        let _res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contracts.raffle.clone(), &RaffleQueryMsg::Config {})
            .unwrap();

        // println!("{:#?}", res);

        // confirm raffle is new owner of nft
        let res: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token.nft.to_string().to_string(),
                &Sg721QueryMsg::OwnerOf {
                    token_id: token.token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, contracts.raffle.clone().to_string());

        // addr_one buys ticket
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(10, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_two buys ticket
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(10, "ustars")],
        };
        buy_tickets_template(params).unwrap();

        // confirm correct number of tickets saved to raffle state
        let res: u32 = app
            .wrap()
            .query_wasm_smart(
                contracts.raffle.clone(),
                &RaffleQueryMsg::TicketCount {
                    owner: one.to_string(),
                    raffle_id: 0,
                },
            )
            .unwrap();
        assert_eq!(res, 1);

        // confirm correct amount of tokens were sent to contract address
        // The creation fee is sent to the treasury address
        // we expect 100000000010 ustars:
        // 0000000000020 - two tickets @ 10 ustars each
        let res = app
            .wrap()
            .query_balance(contracts.raffle.clone(), NATIVE_DENOM.to_string());
        assert_eq!(res.unwrap().amount, Uint128::new(20));

        // The raffle creation fee goes to the treasury
        let res = app
            .wrap()
            .query_balance(TREASURY_ADDR, NATIVE_DENOM.to_string());
        assert_eq!(res.unwrap().amount, Uint128::new(50));

        let owner_balance_before = app
            .wrap()
            .query_balance(owner_addr.clone(), NATIVE_DENOM.to_string())
            .unwrap()
            .amount;
        finish_raffle_timeout(&mut app, &contracts, 0, 130).unwrap();

        // confirm randomness was updated into state
        let res = raffle_info(&app, &contracts, 0);

        assert!(
            res.raffle_info.unwrap().drand_randomness.is_some(),
            "randomness should have been updated into the raffle state"
        );

        let res = raffle_info(&app, &contracts, 0);

        assert_eq!(res.raffle_state, RaffleState::Claimed);

        // confirm the winner exists
        let res: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token.nft.to_string().to_string(),
                &Sg721QueryMsg::OwnerOf {
                    token_id: token.token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, two.to_string());

        // confirm owner of nft is now raffle winner
        let res: ApprovalsResponse = app
            .wrap()
            .query_wasm_smart(
                token.nft.to_string().to_string(),
                &Sg721QueryMsg::Approvals {
                    token_id: token.token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        // confirm nft approval is removed
        assert_eq!(res.approvals, []);

        // confirm raffle owner and treasury set recieve correct amount of tokens
        let owner_balance_after = app
            .wrap()
            .query_balance(owner_addr.clone(), NATIVE_DENOM.to_string())
            .unwrap()
            .amount;
        // at 50%, owner should recieve 10
        assert_eq!(owner_balance_after - owner_balance_before, Uint128::new(10));

        // Treasury should contain 50% (10) + the 50 creation fee
        assert_treasury_balance(&app, NATIVE_DENOM, 10 + 50);
    }

    #[test]
    fn one_raffle_participants() {
        // create testing app
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, one, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);

        create_simple_raffle(&mut app, &contracts, &token, owner_addr.clone());

        // addr_one buys ticket
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(10, "ustars")],
        };
        buy_tickets_template(params).unwrap();

        // confirm correct amount of tokens were sent to contract address
        // we expect 10 ustars:
        // 000000000010 - 1 tickets @ 10 ustars
        let res = app
            .wrap()
            .query_balance(contracts.raffle.clone(), NATIVE_DENOM.to_string());
        assert_eq!(res.unwrap().amount, Uint128::new(10));
        assert_treasury_balance(&app, NATIVE_DENOM, 50);
        let owner_balance_before = app
            .wrap()
            .query_balance(owner_addr.clone(), NATIVE_DENOM.to_string())
            .unwrap()
            .amount;
        finish_raffle_timeout(&mut app, &contracts, 0, 130).unwrap();

        let res = raffle_info(&app, &contracts, 0);

        assert_eq!(res.raffle_state, RaffleState::Claimed);

        // confirm owner of nft is now raffle winner
        let res: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token.nft.to_string().to_string(),
                &Sg721QueryMsg::OwnerOf {
                    token_id: token.token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, one.to_string());

        // confirm treasury has right amount of tokens (creation - 50 + 10% of sales - 5)
        assert_treasury_balance(&app, NATIVE_DENOM, 55);

        // confirm raffle owner and treasury set recieve correct amount of tokens
        let owner_balance_after = app
            .wrap()
            .query_balance(owner_addr.clone(), NATIVE_DENOM.to_string())
            .unwrap()
            .amount;
        // at 50%, owner should recieve 10
        assert_eq!(owner_balance_after - owner_balance_before, Uint128::new(5));
    }

    #[test]
    fn two_raffles_two_participants() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, one, two) = setup_accounts(&mut app);
        let (three, _, _, _, _, _) = setup_raffle_participants(&mut app);

        let token = mint_one_token(&mut app, &contracts);
        let token1 = mint_additional_token(&mut app, &contracts, &token);

        // creates raffle1
        create_simple_raffle(&mut app, &contracts, &token, owner_addr.clone());

        // creates raffle2
        create_simple_raffle(&mut app, &contracts, &token1, owner_addr);

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contracts.raffle.to_string(), &RaffleQueryMsg::Config {})
            .unwrap();
        // confirm raffles were created
        assert_eq!(res.last_raffle_id, 1);

        // purchase raffle tickets
        // addr_one buys 10 tickets from raffle 1
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 10,
            funds_send: vec![coin(100, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_one buys 5 tickets from raffle 2
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 1,
            num_tickets: 5,
            funds_send: vec![coin(50, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_two buys 10 tickets from raffle 2
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 1,
            num_tickets: 10,
            funds_send: vec![coin(100, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_three buys 5 tickets from raffle 2
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![three.clone()],
            raffle_id: 1,
            num_tickets: 5,
            funds_send: vec![coin(50, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_three buys 5 tickets from raffle 1
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![three.clone()],
            raffle_id: 0,
            num_tickets: 5,
            funds_send: vec![coin(50, "ustars")],
        };
        buy_tickets_template(params).unwrap();

        // move forward in time
        let current_time = app.block_info().time;
        let current_block = app.block_info().height;
        let chainid = app.block_info().chain_id.clone();
        app.set_block(BlockInfo {
            height: current_block + 50,
            time: current_time.clone().plus_seconds(1000),
            chain_id: chainid.clone(),
        });

        finish_raffle_timeout(&mut app, &contracts, 0, 1000).unwrap();
        finish_raffle_timeout(&mut app, &contracts, 1, 0).unwrap();
    }
}
