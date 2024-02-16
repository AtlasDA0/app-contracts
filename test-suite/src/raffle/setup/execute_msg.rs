use anyhow::Error as anyhow_error;
use cosmwasm_std::{coin, coins, Addr, Coin, Decimal, Uint128};
use cw_multi_test::{AppResponse, BankSudo, Executor, SudoMsg};
use raffles::{
    msg::{ExecuteMsg as RaffleExecuteMsg, InstantiateMsg, QueryMsg as RaffleQueryMsg},
    state::RaffleOptionsMsg,
};
use sg_std::NATIVE_DENOM;
use utils::state::{AssetInfo, Sg721Token};

use crate::common_setup::{
    contract_boxes::contract_raffles,
    setup_minter::common::constants::{NOIS_PROXY_ADDR, RAFFLE_NAME, RAFFLE_TAX, SG721_CONTRACT},
};

use super::test_msgs::{CreateRaffleParams, InstantiateRaffleParams, PurchaseTicketsParams};

pub fn instantate_raffle_contract(
    params: InstantiateRaffleParams,
) -> Result<cosmwasm_std::Addr, anyhow_error> {
    // define contract instantiation values
    let mut admin_account = params.admin_account;
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
            assets: vec![AssetInfo::Sg721Token(Sg721Token {
                address: SG721_CONTRACT.to_string(),
                token_id: "63".to_string(),
            })],
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
                amount: Uint128::new(4u128),
            }),
            autocycle: Some(false),
        },
        &creation_fee,
    );

    msg
}

pub fn buy_raffle_tickets_template(
    params: PurchaseTicketsParams,
) -> Result<AppResponse, anyhow_error> {
    // define msg values
    let current_time = params.app.block_info().time.clone();
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
            sent_assets: AssetInfo::Coin(Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(64u128),
            }),
        },
        &[Coin {
            denom: NATIVE_DENOM.to_string(),
            amount: Uint128::new(64u128),
        }],
    );
    ticket_purchase1
}
