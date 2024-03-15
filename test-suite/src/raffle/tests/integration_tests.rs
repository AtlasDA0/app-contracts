#[cfg(test)]
mod tests {

    use cosmwasm_std::{coin, Addr, BlockInfo, Coin, Empty, HexBinary, Uint128};
    use cw721::{ApprovalsResponse, OwnerOfResponse};
    use cw_multi_test::Executor;
    use nois::NoisCallback;
    #[cfg(feature = "sg")]
    use sg721_base::QueryMsg as Sg721QueryMsg;
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    use raffles::{
        msg::{ExecuteMsg, QueryMsg as RaffleQueryMsg, RaffleResponse},
        state::{Config, RaffleState, ATLAS_DAO_STARGAZE_TREASURY},
    };

    use crate::{
        common_setup::{
            helpers::setup_block_time,
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_minter::common::constants::{
                FACTORY_ADDR, NOIS_PROXY_ADDR, SG721_CONTRACT, VENDING_MINTER,
            },
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_setup},
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };

    #[test]
    fn two_raffle_participants() {
        // create testing app
        let (mut app, raffle_addr, _) = proper_raffle_instantiate();
        let (owner_addr, one, two) = setup_accounts(&mut app);
        configure_raffle_assets(
            &mut app,
            owner_addr.clone(),
            Addr::unchecked(FACTORY_ADDR),
            true,
        );
        let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(10),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: None,
            gating: vec![],
        };

        // create a raffle
        create_raffle_setup(params);

        let _res: Config = app
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
            funds_send: vec![coin(10, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_two buys ticket
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
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
        let current_time = app.block_info().time;
        let current_block = app.block_info().height;
        let chainid = app.block_info().chain_id.clone();
        setup_block_time(
            &mut app,
            current_time.clone().plus_seconds(130).nanos(),
            Some(current_block + 100),
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

        // confirm owner of nft is now raffle winner
        let res: ApprovalsResponse = app
            .wrap()
            .query_wasm_smart(
                SG721_CONTRACT.to_string(),
                &Sg721QueryMsg::Approvals {
                    token_id: "63".to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        // confirm nft approval is removed
        assert_eq!(res.approvals, []);

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
        let (mut app, raffle_addr, _) = proper_raffle_instantiate();
        let (owner_addr, one, _) = setup_accounts(&mut app);
        configure_raffle_assets(
            &mut app,
            owner_addr.clone(),
            Addr::unchecked(FACTORY_ADDR),
            true,
        );
        let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(10),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: None,
            gating: vec![],
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
            funds_send: vec![coin(10, "ustars")],
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
        let current_time = app.block_info().time;
        let current_block = app.block_info().height;
        let chainid = app.block_info().chain_id.clone();
        setup_block_time(
            &mut app,
            current_time.clone().plus_seconds(130).nanos(),
            Some(current_block + 100),
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
        let res2 = app
            .wrap()
            .query_balance(ATLAS_DAO_STARGAZE_TREASURY, NATIVE_DENOM.to_string());
        // at 50%, owner should recieve 5 & treasury should recieve 5 tokens
        assert_eq!(res1.unwrap().amount, Uint128::new(100000000000005));
        assert_eq!(res2.unwrap().amount, Uint128::new(5));
    }

    #[test]
    fn two_raffles_two_participants() {
        let (mut app, raffle_addr, _) = proper_raffle_instantiate();
        let (owner_addr, one, two) = setup_accounts(&mut app);
        let (three, _, _, _, _, _) = setup_raffle_participants(&mut app);

        // configure nft tokens for raffle 1
        configure_raffle_assets(
            &mut app,
            owner_addr.clone(),
            Addr::unchecked(FACTORY_ADDR),
            true,
        );

        // configure nft tokens for raffle 2
        let mint_nft_tokens = app.execute_contract(
            owner_addr.clone(),
            Addr::unchecked(VENDING_MINTER),
            &vending_minter::msg::ExecuteMsg::Mint {},
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100000u128),
            }],
        );
        assert!(mint_nft_tokens.is_ok());

        // // token id 34
        let _grant_approval = app
            .execute_contract(
                owner_addr.clone(),
                Addr::unchecked(SG721_CONTRACT),
                &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                    spender: raffle_addr.to_string(),
                    token_id: "34".to_string(),
                    expires: None,
                },
                &[],
            )
            .unwrap();

        // creates raffle1
        let params1 = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(10),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: Some(50),
            gating: vec![],
        };
        create_raffle_setup(params1);

        // creates raffle2
        let params2 = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(20),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "34".to_string(),
            })],
            duration: Some(50),
            gating: vec![],
        };
        create_raffle_setup(params2);

        let res: Config = app
            .wrap()
            .query_wasm_smart(raffle_addr.to_string(), &RaffleQueryMsg::Config {})
            .unwrap();
        // confirm raffles were created
        assert_eq!(res.last_raffle_id, Some(1));

        // purchase raffle tickets
        // addr_one buys 10 tickets from raffle 1
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 10,
            funds_send: vec![coin(100, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_one buys 5 tickets from raffle 2
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 1,
            num_tickets: 5,
            funds_send: vec![coin(100, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_two buys 10 tickets from raffle 2
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 1,
            num_tickets: 10,
            funds_send: vec![coin(200, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_three buys 5 tickets from raffle 2
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            msg_senders: vec![three.clone()],
            raffle_id: 1,
            num_tickets: 5,
            funds_send: vec![coin(100, "ustars")],
        };
        buy_tickets_template(params).unwrap();
        // addr_three buys 5 tickets from raffle 1
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
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

        // get randomness for raffle-0
        let _good_receive_randomness = app
            .execute_contract(
                Addr::unchecked(NOIS_PROXY_ADDR),
                raffle_addr.clone(),
                &ExecuteMsg::NoisReceive {
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

        // get randomness for raffle-1
        let _good_receive_randomness = app
            .execute_contract(
                Addr::unchecked(NOIS_PROXY_ADDR),
                raffle_addr.clone(),
                &ExecuteMsg::NoisReceive {
                    callback: NoisCallback {
                        job_id: "raffle-1".to_string(),
                        published: current_time,
                        randomness: HexBinary::from_hex(
                            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa130",
                        )
                        .unwrap(),
                    },
                },
                &[],
            )
            .unwrap();

        // determine winner raffle-0
        // determine the raffle winner, send tokens to winner
        let _claim_ticket = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &ExecuteMsg::DetermineWinner { raffle_id: 0 },
                &[],
            )
            .unwrap();

        // determine winner raffle-1
        // determine the raffle winner, send tokens to winner
        let _claim_ticket = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &ExecuteMsg::DetermineWinner { raffle_id: 1 },
                &[],
            )
            .unwrap();
    }
}
