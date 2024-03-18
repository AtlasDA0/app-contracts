use anyhow::Error as anyhow_error;
use cosmwasm_std::{coin, Coin};
use cw_multi_test::{AppResponse, BankSudo, Executor, SudoMsg};
use raffles::{
    msg::{ExecuteMsg as RaffleExecuteMsg, QueryMsg as RaffleQueryMsg},
    state::RaffleOptionsMsg,
};
use utils::state::AssetInfo;

use crate::common_setup::setup_minter::common::constants::CREATION_FEE_AMNT_STARS;

use super::test_msgs::{CreateRaffleParams, PurchaseTicketsParams};

// Template for creating raffles
pub fn create_raffle_function(params: CreateRaffleParams) -> Result<AppResponse, anyhow_error> {
    // define msg values
    let current_time = params.app.block_info().time;
    let owner_addr = params.owner_addr;
    let raffle_contract = params.raffle_contract_addr;
    let creation_fee = params.creation_fee;
    let assets = params.raffle_nfts;
    let ticket_price = params.ticket_price;

    // fund contract for nois_proxy fee
    params
        .app
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: raffle_contract.clone().to_string(),
                amount: vec![coin(100000000000u128, "ustars".to_string())],
            }
        }))
        .unwrap();
    // create raffle
    params.app.execute_contract(
        owner_addr.clone(),
        raffle_contract.clone(),
        &RaffleExecuteMsg::CreateRaffle {
            owner: None,
            assets,
            raffle_options: RaffleOptionsMsg {
                raffle_start_timestamp: Some(current_time),
                raffle_duration: None,

                comment: None,
                max_ticket_number: None,
                max_ticket_per_address: None,
                raffle_preview: None,
                min_ticket_number: None,
            },
            raffle_ticket_price: AssetInfo::Coin(Coin {
                denom: "ustars".to_string(),
                amount: ticket_price,
            }),
        },
        &creation_fee,
    )
}

pub fn buy_tickets_template(params: PurchaseTicketsParams) -> Result<AppResponse, anyhow_error> {
    // define msg values
    let id = params.raffle_id;
    let msg_sender = &params.msg_senders;
    let raffle_addr = params.raffle_contract_addr;
    let num_tickets = params.num_tickets;
    let funds_sent = params.funds_send;

    // TODO: define # of buyers, return array of address
    // TODO: loop through for each buyer to puchase num_tickets
    params.app.execute_contract(
        msg_sender[0].clone(),
        raffle_addr.clone(),
        &RaffleExecuteMsg::BuyTicket {
            raffle_id: id,
            ticket_count: num_tickets,
            sent_assets: AssetInfo::Coin(funds_sent[0].clone()),
        },
        &funds_sent,
    )
}

pub fn create_raffle_setup(params: CreateRaffleParams) -> anyhow::Result<()> {
    let router = params.app;
    let raffle_addr = params.raffle_contract_addr;
    let owner_addr = params.owner_addr;
    let current_time = router.block_info().time;
    let max_per_addr = params.max_ticket_per_addr;
    let raffle_ticket_price = params.ticket_price;
    let raffle_nfts = params.raffle_nfts;
    let duration = params.duration;

    // create a raffle
    router.execute_contract(
        owner_addr.clone(),
        raffle_addr.clone(),
        &RaffleExecuteMsg::CreateRaffle {
            owner: Some(owner_addr.clone().to_string()),
            assets: raffle_nfts,
            raffle_options: RaffleOptionsMsg {
                raffle_start_timestamp: Some(current_time),
                raffle_duration: duration,

                comment: None,
                max_ticket_number: None,
                max_ticket_per_address: max_per_addr,
                raffle_preview: None,
                min_ticket_number: params.min_ticket_number,
            },
            raffle_ticket_price: AssetInfo::Coin(Coin {
                denom: "ustars".to_string(),
                amount: raffle_ticket_price,
            }),
        },
        &[coin(CREATION_FEE_AMNT_STARS, "ustars")],
    )?;

    let res: raffles::msg::RaffleResponse = router.wrap().query_wasm_smart(
        raffle_addr.clone(),
        &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
    )?;
    assert_eq!(res.clone().raffle_info.unwrap().owner, "owner");
    Ok(())
}

// pub fn determine_winner_template(params: DetermineWinnerParams) -> &mut StargazeApp {
//     let router = params.app;
//     let raffle_addr = params.raffle_contract_addr;
//     let owner_addr = params.owner_addr;
//     let raffle_id = params.raffle_id;

//     let determine_winner = router.execute_contract(
//         owner_addr.clone(),
//         raffle_addr.clone(),
//         &RaffleExecuteMsg::DetermineWinner {
//             raffle_id: raffle_id,
//         },
//         &[],
//     );
//     router
// }
