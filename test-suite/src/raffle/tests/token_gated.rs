#[cfg(test)]
mod tests {
    use crate::common_setup::setup_minter::common::constants::{SG721_CONTRACT, VENDING_MINTER};
    use crate::raffle::setup::daodao::instantiate_with_staked_balances_governance;
    use crate::raffle::setup::{execute_msg::create_raffle_setup, test_msgs::CreateRaffleParams};
    use raffles::state::TokenGatedOptionsMsg;
    use sg_multi_test::StargazeApp;
    use utils::state::{Sg721Token, NATIVE_DENOM};

    use crate::{
        common_setup::{
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{execute_msg::buy_tickets_template, test_msgs::PurchaseTicketsParams},
    };
    use cosmwasm_std::{coin, Addr, Coin, Uint128};
    use cw20::Cw20Coin;
    use cw_multi_test::{BankSudo, Executor, SudoMsg};
    use raffles::msg::QueryMsg as RaffleQueryMsg;
    use std::vec;
    use utils::state::AssetInfo;

    fn create_raffle(
        app: &mut StargazeApp,
        raffle_addr: Addr,
        owner_addr: Addr,
        token_gated: Vec<TokenGatedOptionsMsg>,
    ) {
        let params = CreateRaffleParams {
            app,
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
            token_gated,
        };
        create_raffle_setup(params);
    }
    fn setup_token_gated_raffle(
        token_gated: Vec<TokenGatedOptionsMsg>,
    ) -> (StargazeApp, Addr, Addr) {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);

        create_raffle(
            &mut app,
            raffle_addr.clone(),
            owner_addr.clone(),
            token_gated,
        );
        (app, raffle_addr, one)
    }

    pub const GATED_DENOM: &str = "ugated";
    pub const GATED_DENOM_1: &str = "ugated1";

    #[test]
    fn native_token_gated() {
        let (mut app, raffle_addr, one) =
            setup_token_gated_raffle(vec![TokenGatedOptionsMsg::Coin(coin(12783, GATED_DENOM))]);

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
            raffle_contract_addr: raffle_addr.clone(),
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
                raffle_addr.clone(),
                &RaffleQueryMsg::TicketCount {
                    owner: one.to_string(),
                    raffle_id: 0,
                },
            )
            .unwrap();
        assert_eq!(res, 1);
    }

    #[test]
    fn multiple_native_token_gated() {
        let (mut app, raffle_addr, one) = setup_token_gated_raffle(vec![
            TokenGatedOptionsMsg::Coin(coin(12783, GATED_DENOM)),
            TokenGatedOptionsMsg::Coin(coin(12789, GATED_DENOM_1)),
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
            raffle_contract_addr: raffle_addr.clone(),
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
                raffle_addr.clone(),
                &RaffleQueryMsg::TicketCount {
                    owner: one.to_string(),
                    raffle_id: 0,
                },
            )
            .unwrap();
        assert_eq!(res, 1);
    }

    #[test]
    fn stargaze_token_gated() {
        let (mut app, raffle_addr, one) =
            setup_token_gated_raffle(vec![TokenGatedOptionsMsg::Sg721Token(
                SG721_CONTRACT.to_string(),
            )]);

        app.execute_contract(
            one.clone(),
            Addr::unchecked(VENDING_MINTER),
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
            raffle_contract_addr: raffle_addr.clone(),
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
                raffle_addr.clone(),
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
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);

        let core_addr = instantiate_with_staked_balances_governance(
            &mut app,
            Some(vec![Cw20Coin {
                address: one.to_string(),
                amount: 100_000_000u128.into(),
            }]),
        );

        create_raffle(
            &mut app,
            raffle_addr.clone(),
            owner_addr,
            vec![TokenGatedOptionsMsg::DaoVotingPower {
                dao_address: core_addr.to_string(),
                min_voting_power: 100_000u128.into(),
            }],
        );

        app.execute_contract(
            one.clone(),
            Addr::unchecked(VENDING_MINTER),
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
            raffle_contract_addr: raffle_addr.clone(),
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
                raffle_addr.clone(),
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
        fn bad_native_token_gated() {
            let one_condition = TokenGatedOptionsMsg::Coin(coin(12783, GATED_DENOM));
            let (mut app, raffle_addr, one) = setup_token_gated_raffle(vec![one_condition.clone()]);

            // customize ticket purchase params
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // Buying tickets fails because there are not enough tokens
            buy_tickets_template(params).unwrap_err();
        }

        #[test]
        fn bad_native_token_gated_with_one_success() {
            let (mut app, raffle_addr, one) = setup_token_gated_raffle(vec![
                TokenGatedOptionsMsg::Coin(coin(12783, GATED_DENOM)),
                TokenGatedOptionsMsg::Coin(coin(12789, GATED_DENOM_1)),
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
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // Buying tickets fails because there are not enough tokens
            buy_tickets_template(params).unwrap_err();
        }

        #[test]
        fn bad_stargaze_token_gated() {
            let (mut app, raffle_addr, one) =
                setup_token_gated_raffle(vec![TokenGatedOptionsMsg::Sg721Token(
                    SG721_CONTRACT.to_string(),
                )]);

            // customize ticket purchase params
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
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
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
            configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);

            let core_addr = instantiate_with_staked_balances_governance(&mut app, None);

            create_raffle(
                &mut app,
                raffle_addr.clone(),
                owner_addr,
                vec![TokenGatedOptionsMsg::DaoVotingPower {
                    dao_address: core_addr.to_string(),
                    min_voting_power: 100_000u128.into(),
                }],
            );

            app.execute_contract(
                one.clone(),
                Addr::unchecked(VENDING_MINTER),
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
                raffle_contract_addr: raffle_addr.clone(),
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
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
            configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);

            let core_addr = instantiate_with_staked_balances_governance(
                &mut app,
                Some(vec![Cw20Coin {
                    address: one.to_string(),
                    amount: 1u128.into(),
                }]),
            );

            create_raffle(
                &mut app,
                raffle_addr.clone(),
                owner_addr,
                vec![TokenGatedOptionsMsg::DaoVotingPower {
                    dao_address: core_addr.to_string(),
                    min_voting_power: 100_000u128.into(),
                }],
            );

            app.execute_contract(
                one.clone(),
                Addr::unchecked(VENDING_MINTER),
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
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // simulate the puchase of tickets
            buy_tickets_template(params).unwrap_err();
        }
    }
}
