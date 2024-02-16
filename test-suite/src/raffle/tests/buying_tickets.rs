#[cfg(test)]
mod tests {
    use cosmwasm_std::coin;
    use std::vec;

    use crate::{
        common_setup::{
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_raffle::{
                configure_raffle_assets, create_raffle_setup, proper_raffle_instantiate,
            },
        },
        raffle::setup::{execute_msg::buy_tickets_template, test_msgs::PurchaseTicketsParams},
    };

    #[test]
    fn test_basic_purchase_ticket() {
        let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
        let (owner_address, _, _) = setup_accounts(&mut app);
        let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);
        configure_raffle_assets(&mut app, owner_address.clone(), factory_addr);
        create_raffle_setup(&mut app, raffle_addr.clone(), owner_address.clone());
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
        // println!("{:#?}", purchase_tickets.unwrap());
    }
}
