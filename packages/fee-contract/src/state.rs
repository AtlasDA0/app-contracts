use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ContractInfo {
    pub name: String,
    pub owner: Addr,
    pub p2p_contract: Addr,
    pub fee_distributor: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct FeeInfo {
    pub asset_fee_rate: Uint128, // In thousandths
    pub fee_max: Uint128,        // In uusd
    pub first_tier_limit: Uint128,
    pub first_tier_rate: Uint128,
    pub second_tier_limit: Uint128,
    pub second_tier_rate: Uint128,
    pub third_tier_rate: Uint128,
    pub acceptable_fee_deviation: Uint128, // In thousands
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum FeeType {
    Assets,
    Funds,
}