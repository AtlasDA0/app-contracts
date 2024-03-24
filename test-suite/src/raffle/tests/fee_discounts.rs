#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, HexBinary, Uint128};
    use cw_multi_test::{BankSudo, Executor, SudoMsg};
    use nois::NoisCallback;
    use raffles::{
        error::ContractError,
        msg::{
            ExecuteMsg as RaffleExecuteMsg, InstantiateMsg, QueryMsg as RaffleQueryMsg,
            RaffleResponse,
        },
        state::{RaffleState, StakerFeeDiscount, TREASURY_ADDR},
    };
    use sg721::CollectionInfo;
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM, NOIS_AMOUNT};
    use vending_factory::{
        msg::VendingMinterCreateMsg,
        state::{ParamsExtension, VendingMinterParams},
    };

    use crate::{
        common_setup::{
            app::StargazeApp,
            contract_boxes::custom_mock_app,
            helpers::{assert_error, setup_block_time},
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_minter::common::constants::{NOIS_PROXY_ADDR, OWNER_ADDR, RAFFLE_NAME},
            setup_raffle::raffle_template_code_ids,
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_setup},
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };
    use vending_factory::msg::ExecuteMsg as SgVendingFactoryExecuteMsg;

    pub fn proper_raffle_instantiate(app: &mut StargazeApp, nft_owner: &str) -> (Addr, Addr) {
        let chainid = app.block_info().chain_id.clone();
        setup_block_time(app, 1647032400000000000, Some(10000), &chainid);

        let code_ids = raffle_template_code_ids(app);

        // TODO: setup_factory_template
        let factory_addr = app
            .instantiate_contract(
                code_ids.factory_code_id,
                Addr::unchecked(OWNER_ADDR),
                &vending_factory::msg::InstantiateMsg {
                    params: VendingMinterParams {
                        code_id: code_ids.minter_code_id,
                        allowed_sg721_code_ids: vec![code_ids.sg721_code_id],
                        frozen: false,
                        creation_fee: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100000u128),
                        },
                        min_mint_price: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100000u128),
                        },
                        mint_fee_bps: 10,
                        max_trading_offset_secs: 0,
                        extension: ParamsExtension {
                            max_token_limit: 1000,
                            max_per_address_limit: 20,
                            airdrop_mint_price: Coin {
                                denom: NATIVE_DENOM.to_string(),
                                amount: Uint128::new(100000u128),
                            },
                            airdrop_mint_fee_bps: 10,
                            shuffle_fee: Coin {
                                denom: NATIVE_DENOM.to_string(),
                                amount: Uint128::new(100000u128),
                            },
                        },
                    },
                },
                &[],
                "factory",
                Some(OWNER_ADDR.to_string()),
            )
            .unwrap();

        // We create and mint the atlas NFT collection

        let (stargaze_nft, _token_id) = configure_raffle_assets(
            app,
            Addr::unchecked(nft_owner),
            factory_addr.clone(),
            factory_addr.clone(),
        );

        // create raffle contract
        let raffle_contract_addr = app
            .instantiate_contract(
                code_ids.raffle_code_id,
                Addr::unchecked(OWNER_ADDR),
                &InstantiateMsg {
                    name: RAFFLE_NAME.to_string(),
                    nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
                    nois_proxy_coin: coin(NOIS_AMOUNT, NOIS_DENOM),
                    owner: Some(OWNER_ADDR.to_string()),
                    fee_addr: Some(TREASURY_ADDR.to_owned()),
                    minimum_raffle_duration: None,
                    max_ticket_number: None,
                    raffle_fee: Decimal::percent(50),
                    creation_coins: vec![
                        coin(4, NATIVE_DENOM.to_string()),
                        coin(20, "ustars".to_string()),
                    ]
                    .into(),
                },
                &[],
                "raffle",
                Some(Addr::unchecked(OWNER_ADDR).to_string()),
            )
            .unwrap();

        // fund raffle contract for nois_proxy fee
        app.sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: raffle_contract_addr.clone().to_string(),
                amount: vec![coin(100000000000u128, "ustars".to_string())],
            }
        }))
        .unwrap();

        (raffle_contract_addr, factory_addr)
    }

    #[test]
    pub fn owner_has_nft() {
        let mut app = custom_mock_app();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (raffle_addr, factory_addr) = proper_raffle_instantiate(&mut app, OWNER_ADDR);
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let (nft, token_id) = configure_raffle_assets(
            &mut app,
            owner_addr.clone(),
            factory_addr,
            raffle_addr.clone(),
        );
        // create raffle

        let approvals: cw721::ApprovalsResponse = app
            .wrap()
            .query_wasm_smart(
                nft.clone(),
                &sg721_base::msg::QueryMsg::Approvals {
                    token_id: token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        print!("{:?}", approvals);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: nft.to_string(),
                token_id: token_id.clone(),
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
            Some(current_block + 100),
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
        let res = raffle_info(&app, &contracts, 0);

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
        let res = raffle_info(&app, &contracts, 0);

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
            owner_balance_before.amount
                + Decimal::percent(100) * (Uint128::from(4u128) * Uint128::from(3u128)), // 0% fee for NFT owners
            owner_balance_after.amount
        );
    }

    #[test]
    pub fn owner_is_sufficient_staker() {
        let mut app = custom_mock_app();
        let (owner_addr, one, _) = setup_accounts(&mut app);
        let (raffle_addr, factory_addr) = proper_raffle_instantiate(&mut app, one.as_str());
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let (nft, token_id) = configure_raffle_assets(
            &mut app,
            owner_addr.clone(),
            factory_addr,
            raffle_addr.clone(),
        );

        app.execute(
            owner_addr.clone(),
            cosmwasm_std::CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate {
                validator: "validator".to_string(),
                amount: coin(150, "TOKEN"),
            }),
        )
        .unwrap();
        // create raffle

        let approvals: cw721::ApprovalsResponse = app
            .wrap()
            .query_wasm_smart(
                nft.clone(),
                &sg721_base::msg::QueryMsg::Approvals {
                    token_id: token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        print!("{:?}", approvals);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: nft.to_string(),
                token_id: token_id.clone(),
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
            Some(current_block + 100),
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
        let res = raffle_info(&app, &contracts, 0);

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
        let res = raffle_info(&app, &contracts, 0);

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
            owner_balance_before.amount
                + (Decimal::percent(100) - Decimal::percent(50) * Decimal::percent(50))
                    * (Uint128::from(4u128) * Uint128::from(3u128)), // 50% fee * 50% fee for delegators
            owner_balance_after.amount
        );
    }

    #[test]
    pub fn owner_is_not_sufficient_staker() {
        let mut app = custom_mock_app();
        let (owner_addr, one, _) = setup_accounts(&mut app);
        let (raffle_addr, factory_addr) = proper_raffle_instantiate(&mut app, one.as_str());
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let (nft, token_id) = configure_raffle_assets(
            &mut app,
            owner_addr.clone(),
            factory_addr,
            raffle_addr.clone(),
        );

        app.execute(
            owner_addr.clone(),
            cosmwasm_std::CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate {
                validator: "validator".to_string(),
                amount: coin(50, "TOKEN"),
            }),
        )
        .unwrap();
        // create raffle

        let approvals: cw721::ApprovalsResponse = app
            .wrap()
            .query_wasm_smart(
                nft.clone(),
                &sg721_base::msg::QueryMsg::Approvals {
                    token_id: token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        print!("{:?}", approvals);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: nft.to_string(),
                token_id: token_id.clone(),
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
            Some(current_block + 100),
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
        let res = raffle_info(&app, &contracts, 0);

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
        let res = raffle_info(&app, &contracts, 0);

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
            owner_balance_before.amount
                + (Decimal::percent(100) - Decimal::percent(50))
                    * (Uint128::from(4u128) * Uint128::from(3u128)), // 50% fee fee for everyone
            owner_balance_after.amount
        );
    }
}
