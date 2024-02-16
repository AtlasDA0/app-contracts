use anyhow::Error as anyhow_error;
use cosmwasm_std::{coin, coins, Addr, Decimal};
use cw_multi_test::{BankSudo, Executor, SudoMsg};
use raffles::msg::InstantiateMsg;
use sg_std::NATIVE_DENOM;

use crate::common_setup::{
    contract_boxes::contract_raffles,
    setup_minter::common::constants::{NOIS_PROXY_ADDR, RAFFLE_NAME, RAFFLE_TAX},
};

use super::test_msgs::InstantiateRaffleParams;

pub fn instantate_raffle_contract(
    params: InstantiateRaffleParams,
) -> Result<cosmwasm_std::Addr, anyhow_error> {
    let admin_account = params.admin_account;
    let funds_amount = params.funds_amount;
    let name = params.name;
    let nois_coin = params.nois_proxy_coin;
    let raffle_fee = params.fee_rate;

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
        nois_proxy_addr: NOIS_PROXY_ADDR.into(),
        nois_proxy_coin: nois_coin,
        owner: Some(admin_account.to_string()),
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
