use cosmwasm_std::{to_json_binary, Addr, Deps, QueryRequest, StdError, StdResult, WasmQuery};

use crate::msg::QueryMsg as P2PQueryMsg;
use crate::state::TradeInfo;

/// Load a trade and the provided counter trade
/// If it isn't provided, the function will try to query the accepted counter trade if it exists
pub fn load_trade_and_accepted_counter_trade(
    deps: Deps,
    p2p_contract: Addr,
    trade_id: u64,
    counter_id: Option<u64>,
) -> StdResult<(TradeInfo, TradeInfo)> {
    let trade_info = load_trade(deps, p2p_contract.clone(), trade_id)?;

    let counter_id = match counter_id {
        Some(counter_id) => counter_id,
        None => {
            trade_info
                .clone()
                .accepted_info
                .ok_or_else(|| StdError::generic_err("Trade not accepted"))?
                .counter_id
        }
    };

    let counter_info = load_counter_trade(deps, p2p_contract, trade_id, counter_id)?;

    Ok((trade_info, counter_info))
}

/// Load a trade from the P2P contract
pub fn load_trade(deps: Deps, p2p_contract: Addr, trade_id: u64) -> StdResult<TradeInfo> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: p2p_contract.to_string(),
        msg: to_json_binary(&P2PQueryMsg::TradeInfo { trade_id })?,
    }))
}

/// Load a counter_trade from the P2P contract
pub fn load_counter_trade(
    deps: Deps,
    p2p_contract: Addr,
    trade_id: u64,
    counter_id: u64,
) -> StdResult<TradeInfo> {
    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: p2p_contract.to_string(),
        msg: to_json_binary(&P2PQueryMsg::CounterTradeInfo {
            trade_id,
            counter_id,
        })?,
    }))
}
