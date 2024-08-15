use crate::state::*;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    /// Max # of bundles of nft collections in an infusion
    pub max_bundles: Option<u64>,
    /// Max # of infusion options an infusion may have
    pub max_infusions: Option<u64>,
    /// Max # of tokens to be required per bundle.
    pub max_token_in_bundle: Option<u64>,
    pub cw721_code_id: u64,
}

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    UpdateConfig {
        admin: Option<String>,
        max_infusions: Option<u64>,
        min_infusions_per_bundle: Option<u64>,
        max_infusions_per_bundle: Option<u64>,
    },
    /// Increment count by 1
    CreateInfusion { infusions: Vec<Infusion> },
    // Creates infusion by sending nft tokens
    Infuse {
        infusion_id: u64,
        bundle: Vec<Bundle>,
    },
}

#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    /// returns an infusion for a given infusion owner & infusion id.
    #[returns(Infusion)]
    Infusion { addr: Addr, id: u64 },
    /// returns an infusion for a given infusion id.
    #[returns(Infusion)]
    InfusionById { id: u64 },
    /// returns all infusions owned by a given address
    #[returns(InfusionsResponse)]
    Infusions { addr: Addr },
    /// boolean if collection address is in bundle
    #[returns(bool)]
    IsInBundle { id: u64, collection_addr: Addr },
    /// returns an infused collection address for a given infusion id.
    #[returns(InfusedCollection)]
    InfusedCollection { id: u64 },
}

#[cosmwasm_schema::cw_serde]
pub struct CountResponse {
    pub count: i32,
}

#[cosmwasm_schema::cw_serde]
pub struct InfusedCollectionParams {
    pub code_id: u64,
    pub name: String,
    pub symbol: String,
    pub admin: Option<String>,
}

#[cosmwasm_schema::cw_serde]
pub struct InfusionsResponse {
    pub infusions: Vec<Infusion>,
}
