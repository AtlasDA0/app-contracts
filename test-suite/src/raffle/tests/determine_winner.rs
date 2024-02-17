#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, HexBinary};
    use cw_multi_test::Executor;
    use nois::NoisCallback;
    use raffles::{
        error::ContractError,
        msg::{ExecuteMsg as RaffleExecuteMsg, QueryMsg as RaffleQueryMsg, RaffleResponse},
        state::RaffleState,
    };
    use std::vec;
    use utils::state::NATIVE_DENOM;

    use crate::{
        common_setup::{
            helpers::{assert_error, setup_block_time},
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_minter::common::constants::NOIS_PROXY_ADDR,
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_setup},
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };

    #[test]
    fn test_updating_raffle_randomness() {
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
        // println!("{:#?}", purchase_tickets.unwrap());

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

        // try to claim ticket before randomness is requested
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

        // ensure only nois_proxy provides randomness
        let bad_recieve_randomness = app
            .execute_contract(
                one.clone(),
                raffle_addr.clone(),
                &RaffleExecuteMsg::NoisReceive {
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
            .unwrap_err();
        assert_error(
            Err(bad_recieve_randomness),
            ContractError::UnauthorizedReceive.to_string(),
        );

        // simulates the response from nois_proxy
        let _good_receive_randomness = app
            .execute_contract(
                Addr::unchecked(NOIS_PROXY_ADDR),
                raffle_addr.clone(),
                &RaffleExecuteMsg::NoisReceive {
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
        )
    }
}
