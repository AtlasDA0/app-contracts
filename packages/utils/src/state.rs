use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, to_json_binary, Coin, CosmosMsg as VanillaCosmosMsg, StdResult, Uint128, WasmMsg,
};
use serde::Serialize;
use sg_std::StargazeMsgWrapper;

use sg_std::CosmosMsg;

/// Default limit for proposal pagination.
pub const DEFAULT_LIMIT: u64 = 30;
pub const MAX_COMMENT_SIZE: u64 = 20_000;
pub const RANDOM_BEACON_MAX_REQUEST_TIME_IN_THE_FUTURE: u64 = 7890000;

// ASSETS
#[cfg(feature = "sg")]
#[cw_serde]
pub struct Sg721Token {
    pub address: String,
    pub token_id: String,
}

#[cfg(feature = "sg")]
#[cw_serde]
pub enum AssetInfo {
    Cw721Coin(Cw721Coin),
    Sg721Token(Sg721Token),
    Coin(Coin),
}

#[cfg(feature = "sg")]
impl AssetInfo {
    pub fn coin(amount: u128, denom: &str) -> Self {
        AssetInfo::Coin(coin(amount, denom))
    }

    pub fn coin_raw(amount: Uint128, denom: &str) -> Self {
        AssetInfo::Coin(Coin {
            denom: denom.to_string(),
            amount,
        })
    }

    pub fn cw721(address: &str, token_id: &str) -> Self {
        AssetInfo::Cw721Coin(Cw721Coin {
            address: address.to_string(),
            token_id: token_id.to_string(),
        })
    }
    pub fn sg721(address: &str, token_id: &str) -> Self {
        AssetInfo::Sg721Token(Sg721Token {
            address: address.to_string(),
            token_id: token_id.to_string(),
        })
    }
}

pub type Response = cosmwasm_std::Response<StargazeMsgWrapper>;
pub type SubMsg = cosmwasm_std::SubMsg<StargazeMsgWrapper>;

#[cfg(feature = "sg")]
pub fn into_sg_msg<M: Serialize, T: Into<String>>(
    message: M,
    contract_addr: T,
    funds: Option<Vec<Coin>>,
) -> StdResult<CosmosMsg> {
    let msg = to_json_binary(&message)?;
    let execute = WasmMsg::Execute {
        contract_addr: contract_addr.into(),
        msg,
        funds: funds.unwrap_or_default(),
    };
    Ok(execute.into())
}

#[cw_serde]
pub struct Cw721Coin {
    pub address: String,
    pub token_id: String,
}

#[cw_serde]
pub enum VanillaAssetInfo {
    Cw721Coin(Cw721Coin),
    Coin(Coin),
}

impl VanillaAssetInfo {
    pub fn coin(amount: u128, denom: &str) -> Self {
        VanillaAssetInfo::Coin(coin(amount, denom))
    }

    pub fn coin_raw(amount: Uint128, denom: &str) -> Self {
        VanillaAssetInfo::Coin(Coin {
            denom: denom.to_string(),
            amount,
        })
    }

    pub fn cw721(address: &str, token_id: &str) -> Self {
        VanillaAssetInfo::Cw721Coin(Cw721Coin {
            address: address.to_string(),
            token_id: token_id.to_string(),
        })
    }
}

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 50 {
        return false;
    }
    true
}

pub fn is_valid_comment(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() > 20000 {
        return false;
    }
    true
}

pub fn into_cosmos_msg<M: Serialize, T: Into<String>>(
    message: M,
    contract_addr: T,
    funds: Option<Vec<Coin>>,
) -> StdResult<VanillaCosmosMsg> {
    use serde::Serialize;

    let msg = to_json_binary(&message)?;
    let execute = WasmMsg::Execute {
        contract_addr: contract_addr.into(),
        msg,
        funds: funds.unwrap_or_default(),
    };
    Ok(execute.into())
}
