use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128};
use utils::state::AssetInfo;

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
    pub ticket_price: Uint128,
    pub max_ticket_per_addr: Option<u32>,
    pub raffle_nfts: Vec<AssetInfo>,
    pub duration: Option<u64>,
    pub raffle_start_timestamp: Option<Timestamp>,
}

pub struct PurchaseTicketsParams<'a> {
    pub app: &'a mut sg_multi_test::StargazeApp,
    pub raffle_contract_addr: Addr,
    pub msg_senders: Vec<Addr>,
    pub raffle_id: u64,
    pub num_tickets: u32,
    pub funds_send: Vec<Coin>,
}

// pub struct DetermineWinnerParams<'a> {
//     pub app: &'a mut sg_multi_test::StargazeApp,
//     pub raffle_contract_addr: Addr,
//     pub owner_addr: Addr,
//     pub raffle_id: u64,
// }
