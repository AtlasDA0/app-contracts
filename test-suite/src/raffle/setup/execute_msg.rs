use anyhow::Error as anyhow_error;
use cosmwasm_std::{coin, coins, Coin};
use cw_multi_test::{AppResponse, BankSudo, Executor, SudoMsg};
use raffles::{
    msg::{ExecuteMsg as RaffleExecuteMsg, InstantiateMsg, QueryMsg as RaffleQueryMsg},
    state::RaffleOptionsMsg,
};
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;
use utils::state::AssetInfo;

use crate::common_setup::contract_boxes::contract_raffles;

use super::test_msgs::{CreateRaffleParams, InstantiateRaffleParams, PurchaseTicketsParams};

pub fn instantate_raffle_contract(
    params: InstantiateRaffleParams,
) -> Result<cosmwasm_std::Addr, anyhow_error> {
    // define contract instantiation values
    let admin_account = params.admin_account;
    let funds_amount = params.funds_amount;
    let name = params.name;
    let nois_coin = params.nois_proxy_coin;
    let raffle_fee = params.fee_rate;
    let nois_proxy_addr = params.nois_proxy_addr;

    params
        .app
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: admin_account.to_string(),
                amount: coins(funds_amount, NATIVE_DENOM),
            }
        }))
        .map_err(|err| println!("{err:?}"))
        .ok();

    let raffle_code_id = params.app.store_code(contract_raffles());
    let msg: InstantiateMsg = InstantiateMsg {
        name: name,
        nois_proxy_addr: nois_proxy_addr.to_string(),
        nois_proxy_coin: nois_coin,
        owner: None, // confirm info.sender is default
        fee_addr: Some(admin_account.to_string()),
        minimum_raffle_duration: None,
        minimum_raffle_timeout: None,
        max_ticket_number: None,
        raffle_fee: raffle_fee,
        creation_coins: vec![coin(50, NATIVE_DENOM)].into(),
    };

    params.app.instantiate_contract(
        raffle_code_id,
        admin_account.clone(),
        &msg,
        &coins(funds_amount, NATIVE_DENOM),
        "sg-raffles",
        Some(admin_account.to_string()),
    )
}

// Template for creating raffles
pub fn create_raffle_function(params: CreateRaffleParams) -> Result<AppResponse, anyhow_error> {
    // define msg values
    let current_time = params.app.block_info().time.clone();
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
    let msg = params.app.execute_contract(
        owner_addr.clone(),
        raffle_contract.clone(),
        &RaffleExecuteMsg::CreateRaffle {
            owner: None,
            assets: assets,
            raffle_options: RaffleOptionsMsg {
                raffle_start_timestamp: Some(current_time.clone()),
                raffle_duration: None,
                raffle_timeout: None,
                comment: None,
                max_ticket_number: None,
                max_ticket_per_address: None,
                raffle_preview: None,
            },
            raffle_ticket_price: AssetInfo::Coin(Coin {
                denom: "ustars".to_string(),
                amount: ticket_price,
            }),
            autocycle: Some(false),
        },
        &creation_fee,
    );

    msg
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
    let ticket_purchase1 = params.app.execute_contract(
        msg_sender[0].clone(),
        raffle_addr.clone(),
        &RaffleExecuteMsg::BuyTicket {
            raffle_id: id.clone(),
            ticket_count: num_tickets.clone(),
            sent_assets: AssetInfo::Coin(funds_sent[0].clone()).into(),
        },
        &funds_sent,
    );
    ticket_purchase1
}

pub fn create_raffle_setup(params: CreateRaffleParams) -> &mut StargazeApp {
    let router = params.app;
    let raffle_addr = params.raffle_contract_addr;
    let owner_addr = params.owner_addr;
    let current_time = router.block_info().time.clone();
    let max_per_addr = params.max_ticket_per_addr;
    let raffle_ticket_price = params.ticket_price;
    let raffle_nfts = params.raffle_nfts;
    let duration = params.duration;

    // create a raffle
    let good_create_raffle = router.execute_contract(
        owner_addr.clone(),
        raffle_addr.clone(),
        &RaffleExecuteMsg::CreateRaffle {
            owner: Some(owner_addr.clone().to_string()),
            assets: raffle_nfts,
            raffle_options: RaffleOptionsMsg {
                raffle_start_timestamp: Some(current_time.clone()),
                raffle_duration: duration,
                raffle_timeout: None,
                comment: None,
                max_ticket_number: None,
                max_ticket_per_address: max_per_addr,
                raffle_preview: None,
            },
            raffle_ticket_price: AssetInfo::Coin(Coin {
                denom: "ustars".to_string(),
                amount: raffle_ticket_price,
            }),
            autocycle: Some(false),
        },
        &[coin(4, "ustars")],
    );
    // confirm owner is set
    // assert!(
    //     good_create_raffle.is_ok(),
    //     "There is an issue creating a raffle"
    // );
    good_create_raffle.unwrap();

    let res: raffles::msg::RaffleResponse = router
        .wrap()
        .query_wasm_smart(
            raffle_addr.clone(),
            &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
        )
        .unwrap();
    assert_eq!(res.clone().raffle_info.unwrap().owner, "owner");

    router
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
