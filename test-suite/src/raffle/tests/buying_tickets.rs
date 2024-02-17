#[cfg(test)]
mod tests {
    use crate::common_setup::helpers::assert_error;
    use crate::raffle::setup::{execute_msg::create_raffle_setup, test_msgs::CreateRaffleParams};
    use utils::state::NATIVE_DENOM;

    use crate::{
        common_setup::{
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{execute_msg::buy_tickets_template, test_msgs::PurchaseTicketsParams},
    };
    use cosmwasm_std::{coin, Addr, Coin};
    use cw_multi_test::Executor;
    use raffles::error::ContractError;
    use raffles::msg::{ExecuteMsg as RaffleExecuteMsg, QueryMsg as RaffleQueryMsg};
    use std::vec;
    use utils::state::AssetInfo;

    #[test]
    fn test_basic_purchase_ticket() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: raffle_addr.clone(),
            owner_addr: owner_addr,
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Some(4),
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

        use super::*;

        fn _max_per_address_limit_test() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);
            configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr);

            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_addr,
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: Some(4),
            };
            create_raffle_setup(params);

            // ensure error if max tickets per address set is reached
            let bad_ticket_purchase = app
                .execute_contract(
                    Addr::unchecked("wallet-1"),
                    raffle_addr.clone(),
                    &RaffleExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: 2,
                        sent_assets: AssetInfo::Coin(Coin::new(200, "ustars".to_string())),
                    },
                    &[Coin::new(200, "ustars".to_string())],
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

        fn _end_of_raffle_test() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            let (_, _, _, _, _, _) = setup_raffle_participants(&mut app);
            configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr);
            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_addr,
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: Some(4),
            };
            create_raffle_setup(params);

            
        }
    }
}
