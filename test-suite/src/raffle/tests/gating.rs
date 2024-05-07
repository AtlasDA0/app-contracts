#[cfg(test)]
mod tests {
    use crate::common_setup::app::StargazeApp;
    use crate::common_setup::contract_boxes::contract_cw20;
    use crate::raffle::setup::daodao::instantiate_with_staked_balances_governance;
    use crate::raffle::setup::helpers::mint_one_token;
    use crate::raffle::setup::helpers::TokenMint;
    use crate::raffle::setup::{execute_msg::create_raffle_setup, test_msgs::CreateRaffleParams};
    use crate::{
        common_setup::{
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_raffle::proper_raffle_instantiate,
        },
        raffle::setup::{execute_msg::buy_tickets_template, test_msgs::PurchaseTicketsParams},
    };
    use cosmwasm_std::{coin, Addr, Coin, Uint128};
    use cw20::Cw20Coin;
    use cw_multi_test::{BankSudo, Executor, SudoMsg};
    use raffles::msg::QueryMsg as RaffleQueryMsg;
    use raffles::state::AdvantageOptionsMsg;
    use std::vec;
    use utils::state::AssetInfo;
    use utils::state::{Sg721Token, NATIVE_DENOM};

    fn create_raffle(
        app: &mut StargazeApp,
        raffle: Addr,
        owner_addr: Addr,
        gating: Vec<AdvantageOptionsMsg>,
        token: &TokenMint,
    ) {
        let params = CreateRaffleParams {
            app,
            raffle_contract_addr: raffle.clone(),
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
            min_ticket_number: None,
            max_tickets: None,
            gating,
        };
        create_raffle_setup(params).unwrap();
    }
    fn setup_gating_raffle(gating: Vec<AdvantageOptionsMsg>) -> (StargazeApp, Addr, Addr) {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);

        create_raffle(
            &mut app,
            contracts.raffle.clone(),
            owner_addr.clone(),
            gating,
            &token,
        );
        (app, contracts.raffle, one)
    }

    pub const GATED_DENOM: &str = "ugated";
    pub const GATED_DENOM_1: &str = "ugated1";

    #[test]
    fn native_gating() {
        let (mut app, raffle, one) =
            setup_gating_raffle(vec![AdvantageOptionsMsg::Coin(coin(12783, GATED_DENOM))]);

        app.sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: one.to_string(),
                amount: vec![coin(100_000_000, "ugated".to_string())],
            }
        }))
        .unwrap();
        // customize ticket purchase params
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();

        let res: u32 = app
            .wrap()
            .query_wasm_smart(
                raffle.clone(),
                &RaffleQueryMsg::TicketCount {
                    owner: one.to_string(),
                    raffle_id: 0,
                },
            )
            .unwrap();
        assert_eq!(res, 1);
    }

    #[test]
    fn cw20_gating() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);

        let code_id = app.store_code(contract_cw20());

        let cw20_addr = app
            .instantiate_contract(
                code_id,
                one.clone(),
                &cw20_base::msg::InstantiateMsg {
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: one.to_string(),
                        amount: 100_000_000u128.into(),
                    }],
                    marketing: None,
                    mint: None,
                    symbol: "CWCW".to_string(),
                    name: "cw20".to_string(),
                },
                &[],
                "cw20_example",
                None,
            )
            .unwrap();

        create_raffle(
            &mut app,
            contracts.raffle.clone(),
            owner_addr.clone(),
            vec![AdvantageOptionsMsg::Cw20(Cw20Coin {
                address: cw20_addr.to_string(),
                amount: 13987u128.into(),
            })],
            &token,
        );

        // customize ticket purchase params
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
    }

    #[test]
    fn multiple_native_gating() {
        let (mut app, raffle, one) = setup_gating_raffle(vec![
            AdvantageOptionsMsg::Coin(coin(12783, GATED_DENOM)),
            AdvantageOptionsMsg::Coin(coin(12789, GATED_DENOM_1)),
        ]);

        app.sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: one.to_string(),
                amount: vec![
                    coin(100_000_000, GATED_DENOM.to_string()),
                    coin(100_000_000, GATED_DENOM_1.to_string()),
                ],
            }
        }))
        .unwrap();
        // customize ticket purchase params
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();

        let res: u32 = app
            .wrap()
            .query_wasm_smart(
                raffle.clone(),
                &RaffleQueryMsg::TicketCount {
                    owner: one.to_string(),
                    raffle_id: 0,
                },
            )
            .unwrap();
        assert_eq!(res, 1);
    }

    #[test]
    fn stargaze_gating() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);

        create_raffle(
            &mut app,
            contracts.raffle.clone(),
            owner_addr.clone(),
            vec![AdvantageOptionsMsg::Sg721Token {
                nft_address: token.nft.to_string(),
                nft_count: 1,
            }],
            &token,
        );

        app.execute_contract(
            one.clone(),
            Addr::unchecked(token.minter),
            &vending_minter::msg::ExecuteMsg::Mint {},
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100000u128),
            }],
        )
        .unwrap();

        // customize ticket purchase params
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
    }

    #[test]
    fn dao_gated() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);

        let core_addr = instantiate_with_staked_balances_governance(
            &mut app,
            Some(vec![Cw20Coin {
                address: one.to_string(),
                amount: 100_000_000u128.into(),
            }]),
        );

        create_raffle(
            &mut app,
            contracts.raffle.clone(),
            owner_addr,
            vec![AdvantageOptionsMsg::DaoVotingPower {
                dao_address: core_addr.to_string(),
                min_voting_power: 100_000u128.into(),
            }],
            &token,
        );

        app.execute_contract(
            one.clone(),
            Addr::unchecked(token.minter),
            &vending_minter::msg::ExecuteMsg::Mint {},
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100000u128),
            }],
        )
        .unwrap();

        // customize ticket purchase params
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
    }

    // bad scenarios, expect errors
    mod bad {

        use super::*;

        #[test]
        fn bad_native_gating() {
            let one_condition = AdvantageOptionsMsg::Coin(coin(12783, GATED_DENOM));
            let (mut app, raffle, one) = setup_gating_raffle(vec![one_condition.clone()]);

            // customize ticket purchase params
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // Buying tickets fails because there are not enough tokens
            buy_tickets_template(params).unwrap_err();
        }

        #[test]
        fn bad_cw20_gating() {
            let (mut app, contracts) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
            let token = mint_one_token(&mut app, &contracts);

            let code_id = app.store_code(contract_cw20());

            let cw20_addr = app
                .instantiate_contract(
                    code_id,
                    one.clone(),
                    &cw20_base::msg::InstantiateMsg {
                        decimals: 6,
                        initial_balances: vec![Cw20Coin {
                            address: one.to_string(),
                            amount: 1_000u128.into(),
                        }],
                        marketing: None,
                        mint: None,
                        symbol: "CWCW".to_string(),
                        name: "cw20".to_string(),
                    },
                    &[],
                    "cw20_example",
                    None,
                )
                .unwrap();

            create_raffle(
                &mut app,
                contracts.raffle.clone(),
                owner_addr.clone(),
                vec![AdvantageOptionsMsg::Cw20(Cw20Coin {
                    address: cw20_addr.to_string(),
                    amount: 13987u128.into(),
                })],
                &token,
            );

            // customize ticket purchase params
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: contracts.raffle.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // simulate the puchase of tickets
            let _purchase_tickets = buy_tickets_template(params).unwrap_err();
        }

        #[test]
        fn bad_native_gating_with_one_success() {
            let (mut app, raffle, one) = setup_gating_raffle(vec![
                AdvantageOptionsMsg::Coin(coin(12783, GATED_DENOM)),
                AdvantageOptionsMsg::Coin(coin(12789, GATED_DENOM_1)),
            ]);

            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: one.to_string(),
                    amount: vec![coin(100_000_000, GATED_DENOM_1.to_string())],
                }
            }))
            .unwrap();

            // customize ticket purchase params
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // Buying tickets fails because there are not enough tokens
            buy_tickets_template(params).unwrap_err();
        }

        #[test]
        fn bad_stargaze_gating() {
            let (mut app, contracts) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
            let token = mint_one_token(&mut app, &contracts);

            create_raffle(
                &mut app,
                contracts.raffle.clone(),
                owner_addr.clone(),
                vec![AdvantageOptionsMsg::Sg721Token {
                    nft_address: token.nft.to_string(),
                    nft_count: 1,
                }],
                &token,
            );

            // customize ticket purchase params
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: contracts.raffle.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // simulate the puchase of tickets
            let _purchase_tickets = buy_tickets_template(params).unwrap_err();
        }

        #[test]
        fn no_dao_gated() {
            let (mut app, contracts) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
            let token = mint_one_token(&mut app, &contracts);

            let core_addr = instantiate_with_staked_balances_governance(&mut app, None);

            create_raffle(
                &mut app,
                contracts.raffle.clone(),
                owner_addr,
                vec![AdvantageOptionsMsg::DaoVotingPower {
                    dao_address: core_addr.to_string(),
                    min_voting_power: 100_000u128.into(),
                }],
                &token,
            );

            app.execute_contract(
                one.clone(),
                Addr::unchecked(token.minter),
                &vending_minter::msg::ExecuteMsg::Mint {},
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100000u128),
                }],
            )
            .unwrap();

            // customize ticket purchase params
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: contracts.raffle.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // simulate the puchase of tickets
            buy_tickets_template(params).unwrap_err();
        }

        #[test]
        fn not_enough_dao_gated() {
            let (mut app, contracts) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
            let token = mint_one_token(&mut app, &contracts);

            let core_addr = instantiate_with_staked_balances_governance(
                &mut app,
                Some(vec![Cw20Coin {
                    address: one.to_string(),
                    amount: 1u128.into(),
                }]),
            );

            create_raffle(
                &mut app,
                contracts.raffle.clone(),
                owner_addr,
                vec![AdvantageOptionsMsg::DaoVotingPower {
                    dao_address: core_addr.to_string(),
                    min_voting_power: 100_000u128.into(),
                }],
                &token,
            );

            app.execute_contract(
                one.clone(),
                Addr::unchecked(token.minter),
                &vending_minter::msg::ExecuteMsg::Mint {},
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100000u128),
                }],
            )
            .unwrap();

            // customize ticket purchase params
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: contracts.raffle.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // simulate the puchase of tickets
            buy_tickets_template(params).unwrap_err();
        }
    }
    /// Query tests
    pub mod query {
        use raffles::msg::{AllRafflesResponse, QueryFilters};

        pub use super::*;
        #[test]
        fn multiple_gated_query_works() {
            let (mut app, raffle, one) = setup_gating_raffle(vec![
                AdvantageOptionsMsg::Coin(coin(12783, GATED_DENOM)),
                AdvantageOptionsMsg::Coin(coin(12789, GATED_DENOM_1)),
            ]);

            let partial_conditions_sender = "someone_with_only_some_condition_met".to_string();

            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: one.to_string(),
                    amount: vec![
                        coin(100_000_000, GATED_DENOM.to_string()),
                        coin(100_000_000, GATED_DENOM_1.to_string()),
                    ],
                }
            }))
            .unwrap();
            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: partial_conditions_sender.clone(),
                    amount: vec![coin(100_000_000, GATED_DENOM.to_string())],
                }
            }))
            .unwrap();

            let res: AllRafflesResponse = app
                .wrap()
                .query_wasm_smart(
                    raffle.clone(),
                    &RaffleQueryMsg::AllRaffles {
                        start_after: None,
                        limit: None,
                        filters: Some(QueryFilters {
                            states: None,
                            owner: None,
                            ticket_depositor: None,
                            contains_token: None,
                            gated_rights_ticket_buyer: Some(one.to_string()),
                        }),
                    },
                )
                .unwrap();
            assert_eq!(res.raffles.len(), 1);

            let res: AllRafflesResponse = app
                .wrap()
                .query_wasm_smart(
                    raffle.clone(),
                    &RaffleQueryMsg::AllRaffles {
                        start_after: None,
                        limit: None,
                        filters: Some(QueryFilters {
                            states: None,
                            owner: None,
                            ticket_depositor: None,
                            contains_token: None,
                            gated_rights_ticket_buyer: Some(partial_conditions_sender.clone()),
                        }),
                    },
                )
                .unwrap();
            assert_eq!(res.raffles.len(), 0);

            let res: AllRafflesResponse = app
                .wrap()
                .query_wasm_smart(
                    raffle.clone(),
                    &RaffleQueryMsg::AllRaffles {
                        start_after: None,
                        limit: None,
                        filters: Some(QueryFilters {
                            states: None,
                            owner: None,
                            ticket_depositor: None,
                            contains_token: None,
                            gated_rights_ticket_buyer: Some(
                                "someone-with-no-conditions-met".to_string(),
                            ),
                        }),
                    },
                )
                .unwrap();
            assert_eq!(res.raffles.len(), 0);
        }
    }
}
