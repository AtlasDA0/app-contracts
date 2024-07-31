use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cosmwasm_schema::cw_serde]
pub struct Config {
    pub latest_infusion_id: Option<u64>,
    pub admin: Addr,
    pub max_infusions: u64,
    /// minimum nfts bundles must require
    pub min_per_bundle: u64,
    /// maximum nfts bundles can require
    pub max_per_bundle: u64,
    /// maximum bundles allowed per infusion
    pub max_bundles: u64,
    /// cw721-base code_id
    pub code_id: u64,
}

#[cosmwasm_schema::cw_serde]
pub struct Infusion {
    pub collections: Vec<NFTCollection>,
    pub infused_collection: InfusedCollection,
    pub infusion_params: InfusionParams,
    pub infusion_id: u64,
}

pub const CONFIG: Item<Config> = Item::new("cfg");
pub const COUNT: Item<i32> = Item::new("cnt");
pub const INFUSION: Map<(Addr, u64), Infusion> = Map::new("i");
pub const INFUSION_ID: Map<u64, (Addr, u64)> = Map::new("iid");
pub const INFUSION_INFO: Map<&Addr, InfusionInfo> = Map::new("ii");

#[cosmwasm_schema::cw_serde]
pub struct InfusionParams {
    pub params: BurnParams,
}

#[cosmwasm_schema::cw_serde]
pub struct NFT {
    pub addr: Addr,
    pub token_id: u64,
}

#[cosmwasm_schema::cw_serde]
pub struct NFTCollection {
    pub addr: Addr,
    /// # of tokens required from this collection
    pub min_wanted: u64,
    pub max: Option<u64>,
}

#[cosmwasm_schema::cw_serde]
pub struct InfusedCollection {
    pub addr: Addr,
    pub admin: Option<String>,
    pub name: String,
    pub symbol: String,
}

#[cosmwasm_schema::cw_serde]
pub struct BurnParams {
    pub compatible_traits: Option<CompatibleTraits>,
}

#[cosmwasm_schema::cw_serde]
pub struct CompatibleTraits {
    pub a: String,
    pub b: String,
}

#[cosmwasm_schema::cw_serde]
pub struct Bundle {
    pub nfts: Vec<NFT>,
}
#[cosmwasm_schema::cw_serde]
#[derive(Default)]
pub struct InfusionInfo {
    pub next_id: u64,
}
