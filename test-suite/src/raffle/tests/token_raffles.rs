#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Uint128};
    use raffles::{error::ContractError, state::RaffleState};
    use std::vec;
    use utils::state::{AssetInfo, NATIVE_DENOM};

    use crate::{
        common_setup::{
            helpers::{assert_error, setup_block_time},
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_raffle::{
                proper_raffle_instantiate, proper_raffle_instantiate_precise, DRAND_TIMEOUT,
            },
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_setup},
            helpers::{
                finish_raffle_timeout, mint_one_token, raffle_info, send_update_randomness_message,
            },
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };

    #[test]
    fn test_multiple_tickets_token_raffle() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create raffle

        let raffle_amount = 1898u128;
        let raffled_denom = "uraffled";

        let raffled_assets = coin(raffle_amount, raffled_denom);

        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: owner_addr.to_string(),
                amount: vec![raffled_assets.clone()],
            },
        ))
        .unwrap();

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Coin(raffled_assets)],
            duration: None,
            min_ticket_number: None,
            max_tickets: None,
            gating: vec![],
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
        let claim_but_no_randomness_yet =
            send_update_randomness_message(&mut app, &contracts, 0, 0).unwrap_err();

        assert_error(
            Err(claim_but_no_randomness_yet),
            ContractError::WrongStateForRandomness {
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
        finish_raffle_timeout(&mut app, &contracts, 0, 0 + DRAND_TIMEOUT).unwrap();

        // queries the raffle
        let res = raffle_info(&app, &contracts, 0).raffle_info.unwrap();
        // verify randomness state has been updated
        assert!(
            res.drand_randomness.is_some(),
            "randomness should have been updated into the raffle state"
        );

        // verify winner is always owner
        assert_eq!(
            two, res.winners[0],
            "Winner should be the 2nd one with this randomness"
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

        assert_eq!(
            app.wrap()
                .query_balance(&owner_addr, raffled_denom)
                .unwrap()
                .amount
                .u128(),
            0
        );

        assert_eq!(
            app.wrap()
                .query_balance(&res.winners[0], raffled_denom)
                .unwrap()
                .amount
                .u128(),
            raffle_amount
        );

        assert_eq!(
            app.wrap()
                .query_balance(&one, raffled_denom)
                .unwrap()
                .amount
                .u128(),
            0
        );

        assert_eq!(
            app.wrap()
                .query_balance(&three, raffled_denom)
                .unwrap()
                .amount
                .u128(),
            0
        );
    }
}
