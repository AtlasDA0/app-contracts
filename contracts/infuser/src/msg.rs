use crate::state::*;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub max_bundles: Option<u64>,
    pub max_infusions: Option<u64>,
    pub max_token_in_bundle: Option<u64>,
    pub cw721_code_id: u64,
}

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    UpdateConfig {},
    /// Increment count by 1
    CreateInfusion {
        collections: Vec<Infusion>,
    },
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
    IsInBundle { collection_addr: Addr },
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
