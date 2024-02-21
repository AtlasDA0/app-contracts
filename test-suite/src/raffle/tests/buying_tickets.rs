#[cfg(test)]
mod tests {
    use crate::common_setup::helpers::assert_error;
    use crate::common_setup::setup_minter::common::constants::SG721_CONTRACT;
    use crate::raffle::setup::{execute_msg::create_raffle_setup, test_msgs::CreateRaffleParams};
    use utils::state::{Sg721Token, NATIVE_DENOM};

    use crate::{
        common_setup::{
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{execute_msg::buy_tickets_template, test_msgs::PurchaseTicketsParams},
    };
    use cosmwasm_std::{coin, Coin, Uint128};
    use cw_multi_test::Executor;
    use raffles::error::ContractError;
    use raffles::msg::{ExecuteMsg as RaffleExecuteMsg, QueryMsg as RaffleQueryMsg};
    use std::vec;
    use utils::state::AssetInfo;

    #[test]
    fn basic_ticket_purchase() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr,
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
            duration: None,
        };
        create_raffle_setup(params);
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
        let purchase_tickets = buy_tickets_template(params);
        assert!(
            purchase_tickets.is_ok(),
            "There is an issue with purchasing a ticket"
        );
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
        // println!("{:#?}", purchase_tickets.unwrap());
    }

    // bad scenarios, expect errors
    mod bad {

        use cosmwasm_std::Uint128;
        use utils::state::Sg721Token;

        use crate::raffle::setup::execute_msg::create_raffle_function;

        use super::*;

        #[test]
        fn _max_per_address_limit_test() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_addr, one, _) = setup_accounts(&mut app);
            let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);
            configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr.clone(), true);

            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_addr,
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: Uint128::new(4),
                max_ticket_per_addr: Some(1),
                raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                    address: SG721_CONTRACT.to_string(),
                    token_id: "63".to_string(),
                })],
                duration: None,
            };
            create_raffle_setup(params);

            // ensure error if max tickets per address set is reached
            let bad_ticket_purchase = app
                .execute_contract(
                    one.clone(),
                    raffle_addr.clone(),
                    &RaffleExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: 2,
                        sent_assets: AssetInfo::Coin(Coin::new(8, "ustars".to_string())),
                    },
                    &[Coin::new(8, "ustars".to_string())],
                )
                .unwrap_err();
            assert_error(
                Err(bad_ticket_purchase),
                ContractError::TooMuchTicketsForUser {
                    max: 1,
                    nb_before: 0,
                    nb_after: 2,
                }
                .to_string(),
            );
        }

        // #[test]
        // fn _end_of_raffle_test() {
        //     let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        //     let (owner_addr, _, _) = setup_accounts(&mut app);
        //     let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);
        //     configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);

        //     let params = CreateRaffleParams {
        //         app: &mut app,
        //         raffle_contract_addr: raffle_addr.clone(),
        //         owner_addr: owner_addr,
        //         creation_fee: vec![coin(4, NATIVE_DENOM)],
        //         ticket_price: Some(4),
        //         max_ticket_per_addr: None,
        //         raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
        //            address: SG721_CONTRACT.to_string(),
        //             token_id: "63".to_string(),
        //         })],
        //     };
        //     create_raffle_setup(params);
        // }

        #[test]
        fn bad_ticket_sale_no_funds_provided() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_address, one, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_address.clone(), factory_addr, true);
            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_address.clone(),
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: Uint128::new(4),
                max_ticket_per_addr: None,
                raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                    address: SG721_CONTRACT.to_string(),
                    token_id: "63".to_string(),
                })],
                duration: None,
            };

            create_raffle_function(params).unwrap();

            let invalid_raffle_purchase = app
                .execute_contract(
                    one.clone(),
                    raffle_addr.clone(),
                    &RaffleExecuteMsg::BuyTicket {
                        raffle_id: 1,
                        ticket_count: 1,
                        sent_assets: AssetInfo::Coin(Coin {
                            denom: "ustars".to_string(),
                            amount: Uint128::new(69u128),
                        }),
                    },
                    &[],
                )
                .unwrap_err();
            assert_error(
                Err(invalid_raffle_purchase),
                ContractError::AssetMismatch {}.to_string(),
            );
        }

        #[test]
        fn bad_raffle_id() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_addr, one, _) = setup_accounts(&mut app);
            let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);
            configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);
            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_addr,
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: Uint128::new(4),
                max_ticket_per_addr: None,
                raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                    address: SG721_CONTRACT.to_string(),
                    token_id: "63".to_string(),
                })],
                duration: None,
            };
            create_raffle_setup(params);

            let bad_ticket_purchase = app.execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &RaffleExecuteMsg::BuyTicket {
                    raffle_id: 1,
                    ticket_count: 2,
                    sent_assets: AssetInfo::Coin(Coin::new(8, "ustars".to_string())),
                },
                &[Coin::new(8, "ustars".to_string())],
            );
            assert!(bad_ticket_purchase.is_err());
        }

        #[test]
        fn bad_payment_amount_() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_addr, one, _) = setup_accounts(&mut app);
            let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);

            // bad params
            let ticket_count = 2u32;
            let sent_coin = Coin::new(20, "ustars".to_string());
            let sent_assets = AssetInfo::Coin(sent_coin.clone());
            let assets_wanted = AssetInfo::Coin(Coin::new(
                (ticket_count.clone() * ticket_count.clone()).into(),
                "ustars",
            ));

            configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr, true);
            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_addr,
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: Uint128::new(4),
                max_ticket_per_addr: None,
                raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                    address: SG721_CONTRACT.to_string(),
                    token_id: "63".to_string(),
                })],
                duration: None,
            };
            create_raffle_setup(params);
            // Too many tokens sent
            let bad_ticket_purchase = app
                .execute_contract(
                    one.clone(),
                    raffle_addr.clone(),
                    &RaffleExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: ticket_count.clone(),
                        sent_assets: sent_assets,
                    },
                    &[sent_coin.clone()],
                )
                .unwrap_err();
            assert_error(
                Err(bad_ticket_purchase),
                ContractError::PaymentNotSufficient {
                    ticket_count: ticket_count.clone(),
                    assets_wanted: assets_wanted.clone(),
                    assets_received: utils::state::AssetInfo::Coin(sent_coin),
                }
                .to_string(),
            );
            // Too few tokens sent
            let sent_coin = Coin::new(2, "ustars".to_string());
            let sent_assets = AssetInfo::Coin(sent_coin.clone());

            let bad_ticket_purchase = app
                .execute_contract(
                    one.clone(),
                    raffle_addr.clone(),
                    &RaffleExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: ticket_count.clone(),
                        sent_assets: sent_assets.clone(),
                    },
                    &[sent_coin.clone()],
                )
                .unwrap_err();
            assert_error(
                Err(bad_ticket_purchase),
                ContractError::PaymentNotSufficient {
                    ticket_count: ticket_count.clone(),
                    assets_wanted: assets_wanted.clone(),
                    assets_received: utils::state::AssetInfo::Coin(sent_coin),
                }
                .to_string(),
            );

            // sent_assets not true
            let sent_coin = Coin::new(4, "ustars".to_string());
            let sent_assets = AssetInfo::Coin(Coin::new(8, "ustars".to_string()));

            let bad_ticket_purchase = app
                .execute_contract(
                    one.clone(),
                    raffle_addr.clone(),
                    &RaffleExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: ticket_count.clone(),
                        sent_assets: sent_assets.clone(),
                    },
                    &[sent_coin.clone()],
                )
                .unwrap_err();
            assert_error(
                Err(bad_ticket_purchase),
                ContractError::AssetMismatch {}.to_string(),
            );
        }
    }
}
