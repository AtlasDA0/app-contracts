use std::{ops::Add, vec};

use cosmwasm_std::{Addr, Coin, Decimal, Uint128};

pub struct InstantiateRaffleParams<'a> {
    pub app: &'a mut sg_multi_test::StargazeApp,
    pub admin_account: Addr,
    pub funds_amount: u128,
    pub fee_rate: Decimal,
    pub name: String,
    pub nois_proxy_coin: Coin,
    pub nois_proxy_addr: String,
}

pub struct CreateRaffleParams<'a> {
    pub app: &'a mut sg_multi_test::StargazeApp,
    pub raffle_contract_addr: Addr,
    pub owner_addr: Addr,
    pub creation_fee: Vec<Coin>,
    pub ticket_price: Option<u128>,
}

pub struct PurchaseTicketsParams<'a> {
    pub app: &'a mut sg_multi_test::StargazeApp,
    pub raffle_contract_addr: Addr,
    pub msg_senders: Vec<Addr>,
    pub raffle_id: u64,
    pub num_tickets: u32,
    pub funds_send: Vec<Coin>,
}
