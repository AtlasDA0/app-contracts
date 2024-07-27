use cosmwasm_std::{Binary, Decimal};
use strum_macros;

use cosmwasm_std::{Addr, Coin, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utils::state::AssetInfo;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, strum_macros::Display)]
#[serde(rename_all = "snake_case")]
pub enum TradeState {
    Created,
    Published,
    Countered,
    Refused,
    Accepted,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ContractInfo {
    pub name: String,
    pub owner: Addr,
    pub accept_trade_fee: Vec<Coin>,
    pub fund_fee: Decimal,
    pub treasury: Addr,
    pub last_trade_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct CounterTradeInfo {
    pub trade_id: u64,
    pub counter_id: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct AdditionalTradeInfo {
    pub time: Timestamp,
    pub owner_comment: Option<Comment>,
    pub trader_comment: Option<Comment>,
    pub nfts_wanted: HashSet<Addr>,
    pub tokens_wanted: HashSet<Binary>, // The tokens wanted can only be a coin of a cw20
    pub trade_preview: Option<AssetInfo>, // The preview can only be a CW1155 or a CW721 token.
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct Comment {
    pub time: Timestamp,
    pub comment: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TradeInfo {
    pub owner: Addr,
    pub associated_assets: Vec<AssetInfo>,
    pub state: TradeState,
    pub last_counter_id: Option<u64>,
    pub whitelisted_users: HashSet<Addr>,
    pub additional_info: AdditionalTradeInfo,
    pub accepted_info: Option<CounterTradeInfo>,
    pub assets_withdrawn: bool,
}

impl Default for TradeInfo {
    fn default() -> Self {
        Self {
            owner: Addr::unchecked(""),
            associated_assets: vec![],
            state: TradeState::Created,
            last_counter_id: None,
            whitelisted_users: HashSet::new(),
            additional_info: AdditionalTradeInfo::default(),
            accepted_info: None,
            assets_withdrawn: false,
        }
    }
}
