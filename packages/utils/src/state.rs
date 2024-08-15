use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, to_json_binary, Coin, StdResult, Uint128, WasmMsg};
use serde::Serialize;

use crate::types::CosmosMsg;

/// Default limit for proposal pagination.
pub const DEFAULT_LIMIT: u64 = 30;
pub const MAX_COMMENT_SIZE: u64 = 20_000;
pub const RANDOM_BEACON_MAX_REQUEST_TIME_IN_THE_FUTURE: u64 = 7890000; // 3 months
pub const NATIVE_DENOM: &str = "ustars"; // TODO: Setup native tokens repo

// ASSETS
#[cw_serde]
#[cfg(feature = "sg")]
pub struct Sg721Token {
    pub address: String,
    pub token_id: String,
}

#[cw_serde]
pub struct Cw721Coin {
    pub address: String,
    pub token_id: String,
}

#[cfg(feature = "sg")]
#[cw_serde]
pub enum AssetInfo {
    Cw721Coin(Cw721Coin),
    Coin(Coin),
    Sg721Token(Sg721Token),
}

impl AssetInfo {
    pub fn overlaps(&self, asset: &AssetInfo) -> bool {
        match self {
            AssetInfo::Coin(_) => false,
            AssetInfo::Sg721Token(Sg721Token { address, token_id })
            | AssetInfo::Cw721Coin(Cw721Coin { address, token_id }) => match asset {
                AssetInfo::Coin(_) => false,
                AssetInfo::Sg721Token(Sg721Token {
                    address: address1,
                    token_id: token_id1,
                })
                | AssetInfo::Cw721Coin(Cw721Coin {
                    address: address1,
                    token_id: token_id1,
                }) => address == address1 && token_id1 == token_id,
            },
        }
    }
    pub fn debug_str(&self) -> String {
        match self {
            AssetInfo::Coin(c) => c.denom.to_string(),
            AssetInfo::Sg721Token(Sg721Token { address, token_id })
            | AssetInfo::Cw721Coin(Cw721Coin { address, token_id }) => {
                format!("{}-{}", address, token_id)
            }
        }
    }
}

#[cfg(not(feature = "sg"))]
#[cw_serde]
pub enum AssetInfo {
    Cw721Coin(Cw721Coin),
    Coin(Coin),
}

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
    #[cfg(feature = "sg")]
    pub fn sg721(address: &str, token_id: &str) -> Self {
        AssetInfo::Sg721Token(Sg721Token {
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
pub enum LoanSudoMsg {
    ToggleLock { lock: bool },
}

#[cw_serde]
pub enum RaffleSudoMsg {
    ToggleLock { lock: bool },
    BeginBlock {},
}

#[cw_serde]
pub struct Locks {
    pub lock: bool,
    pub sudo_lock: bool,
}

pub fn all_elements_unique(vec: &[AssetInfo]) -> bool {
    for i in 0..vec.len() {
        for j in i + 1..vec.len() {
            if vec[i].overlaps(&vec[j]) {
                return false;
            }
        }
    }
    true
}

pub fn dedupe(vec: &[AssetInfo]) -> Vec<AssetInfo> {
    let mut all_unique = vec![];
    'outer: for i in 0..vec.len() {
        for j in i + 1..vec.len() {
            if vec[i].overlaps(&vec[j]) {
                continue 'outer;
            }
        }
        all_unique.push(vec[i].clone())
    }
    all_unique
}
