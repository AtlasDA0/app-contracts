use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};
use raffles::msg::GatingOptionsMsg;
use utils::state::AssetInfo;

pub struct CreateRaffleParams<'a> {
    pub app: &'a mut sg_multi_test::StargazeApp,
    pub raffle_contract_addr: Addr,
    pub owner_addr: Addr,
    pub creation_fee: Vec<Coin>,
    pub ticket_price: Uint128,
    pub max_ticket_per_addr: Option<u32>,
    pub max_tickets: Option<u32>,
    pub min_ticket_number: Option<u32>,
    pub raffle_nfts: Vec<AssetInfo>,
    pub duration: Option<u64>,
    pub raffle_start_timestamp: Option<Timestamp>,
    pub gating: Vec<GatingOptionsMsg>,
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
