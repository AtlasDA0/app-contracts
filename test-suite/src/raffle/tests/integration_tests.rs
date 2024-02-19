#[cfg(test)]
mod tests {

    use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, HexBinary, Timestamp, Uint128};
    use cw721::OwnerOfResponse;
    use cw_multi_test::Executor;
    use nois::NoisCallback;
    #[cfg(feature = "sg")]
    use sg721_base::QueryMsg as Sg721QueryMsg;
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    use raffles::{
        error::ContractError,
        msg::{ExecuteMsg, InstantiateMsg, QueryMsg as RaffleQueryMsg, RaffleResponse},
        state::{Config, RaffleOptionsMsg, RaffleState, ATLAS_DAO_STARGAZE_TREASURY, NOIS_AMOUNT},
    };

    use crate::{
        common_setup::{
            helpers::{assert_error, setup_block_time},
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_minter::common::constants::{
                FACTORY_ADDR, MINT_PRICE, NOIS_PROXY_ADDR, OWNER_ADDR, RAFFLE_NAME, RAFFLE_TAX,
                SG721_CONTRACT,
            },
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{
            execute_msg::{
                buy_tickets_template, create_raffle_function, create_raffle_setup,
                instantate_raffle_contract,
            },
            test_msgs::{CreateRaffleParams, InstantiateRaffleParams, PurchaseTicketsParams},
        },
    };

    #[test]
    fn two_raffle_participants() {
        // create testing app
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, one, two) = setup_accounts(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), Addr::unchecked(FACTORY_ADDR));
        let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Some(10),
            max_ticket_per_addr: None,
        };

        // create a raffle
        create_raffle_setup(params);

        let res: Config = app
            .wrap()
            .query_wasm_smart(raffle_addr.clone(), &RaffleQueryMsg::Config {})
            .unwrap();

        // println!("{:#?}", res);

        // confirm raffle is new owner of nft
        let res: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                SG721_CONTRACT.to_string(),
                &Sg721QueryMsg::OwnerOf {
                    token_id: "63".to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, raffle_addr.clone().to_string());

        // addr_one buys ticket
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(10, "ustars")].into(),
        };
        buy_tickets_template(params).unwrap();
        // addr_two buys ticket
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(10, "ustars")].into(),
        };
        buy_tickets_template(params).unwrap();

        // confirm correct number of tickets saved to raffle state
        let res: u32 = app
            .wrap()
            .query_wasm_smart(
                raffle_addr.clone(),
                &RaffleQueryMsg::TicketCount {
                    owner: one.to_string(),
                    raffle_id: 0,
                },
            )
            .unwrap();
        assert_eq!(res, 1);

        // confirm correct amount of tokens were sent to contract address
        // we expect 100000000012 ustars:
        // 100000000000 - initial balance of raffle contract
        // 000000000004 - single raffle creation fee
        // 0000000000020 - two tickets @ 10 ustars each
        let res = app
            .wrap()
            .query_balance(raffle_addr.clone(), NATIVE_DENOM.to_string());
        assert_eq!(res.unwrap().amount, Uint128::new(100000000024));

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

        // simulates the response from nois_proxy
        let _good_receive_randomness = app
            .execute_contract(
                Addr::unchecked(NOIS_PROXY_ADDR),
                raffle_addr.clone(),
                &ExecuteMsg::NoisReceive {
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
        // confirm randomness was updated into state
        let res: RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_addr.clone(),
                &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();

        assert!(
            res.raffle_info.unwrap().randomness.is_some(),
            "randomness should have been updated into the raffle state"
        );

        // determine the raffle winner, send tokens to winner
        let _claim_ticket = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &ExecuteMsg::DetermineWinner { raffle_id: 0 },
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
        assert_eq!(res.raffle_state, RaffleState::Claimed);

        // println!("{:#?}", _claim_ticket);

        // confirm owner of nft is now raffle winner
        let res: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                SG721_CONTRACT.to_string(),
                &Sg721QueryMsg::OwnerOf {
                    token_id: "63".to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, two.to_string());

        // confirm raffle owner and treasury set recieve correct amount of tokens
        let owner_balance = app
            .wrap()
            .query_balance(owner_addr.clone(), NATIVE_DENOM.to_string());
        let treasury_balance = app
            .wrap()
            .query_balance(ATLAS_DAO_STARGAZE_TREASURY, NATIVE_DENOM.to_string());
        // at 50%, owner should recieve 10 & treasury should recieve 10 tokens
        assert_eq!(owner_balance.unwrap().amount, Uint128::new(100000000000010));
        assert_eq!(treasury_balance.unwrap().amount, Uint128::new(10));
    }

    #[test]
    fn one_raffle_participants() {
        // create testing app
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, one, two) = setup_accounts(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), Addr::unchecked(FACTORY_ADDR));
        let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Some(10),
            max_ticket_per_addr: None,
        };

        // create a raffle
        create_raffle_setup(params);

        // addr_one buys ticket
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(10, "ustars")].into(),
        };
        buy_tickets_template(params).unwrap();

        // confirm correct amount of tokens were sent to contract address
        // we expect 100000000012 ustars:
        // 100000000000 - initial balance of raffle contract
        // 000000000004 - single raffle creation fee
        // 000000000010 - 1 tickets @ 10 ustars
        let res = app
            .wrap()
            .query_balance(raffle_addr.clone(), NATIVE_DENOM.to_string());
        assert_eq!(res.unwrap().amount, Uint128::new(100000000014));

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

        // simulates the response from nois_proxy
        let _good_receive_randomness = app
            .execute_contract(
                Addr::unchecked(NOIS_PROXY_ADDR),
                raffle_addr.clone(),
                &ExecuteMsg::NoisReceive {
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

        // determine the raffle winner, send tokens to winner
        let _claim_ticket = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &ExecuteMsg::DetermineWinner { raffle_id: 0 },
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
        assert_eq!(res.raffle_state, RaffleState::Claimed);

        // confirm owner of nft is now raffle winner
        let res: OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                SG721_CONTRACT.to_string(),
                &Sg721QueryMsg::OwnerOf {
                    token_id: "63".to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, one.to_string());

        // confirm raffle owner and treasury set recieve correct amount of tokens
        let res1 = app
            .wrap()
            .query_balance(owner_addr.clone(), NATIVE_DENOM.to_string());
        let res2 = app.wrap().query_balance(
            ATLAS_DAO_STARGAZE_TREASURY.clone(),
            NATIVE_DENOM.to_string(),
        );
        // at 50%, owner should recieve 5 & treasury should recieve 5 tokens
        assert_eq!(res1.unwrap().amount, Uint128::new(100000000000005));
        assert_eq!(res2.unwrap().amount, Uint128::new(5));
    }
    #[test]
    fn free_tickets() {}
}
