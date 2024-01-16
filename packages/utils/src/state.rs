use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Uint128, coin, StdResult, to_json_binary, WasmMsg};
use serde::Serialize;
use sg_std::CosmosMsg;


// ASSETS
#[cw_serde]
pub struct Sg721Token {
    pub address: String,
    pub token_id: String,
}

#[cw_serde]
pub struct Cw721Coin {
    pub address: String,
    pub token_id: String,
}

#[cw_serde]
pub enum AssetInfo<> {
    Cw721Coin(Cw721Coin),
    Sg721Token(Sg721Token),
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