use crate::ContractError;
use cosmwasm_std::Api;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Deps, Order, StdResult, Storage};
use std::convert::TryInto;

use cw_storage_plus::Bound;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{
    get_actual_counter_state, load_counter_trade, load_trade, CONTRACT_INFO, COUNTER_TRADE_INFO,
    TRADE_INFO,
};
use p2p_trading_export::msg::{QueryFilters, TradeInfoResponse};
use p2p_trading_export::state::{AssetInfo, ContractInfo, CounterTradeInfo, TradeInfo};

use itertools::Itertools;
// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
const BASE_LIMIT: usize = 100;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct TradeResponse {
    pub trade_id: u64,
    pub counter_id: Option<u64>,
    pub trade_info: Option<TradeInfoResponse>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AllTradesResponse {
    pub trades: Vec<TradeResponse>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AllCounterTradesResponse {
    pub counter_trades: Vec<TradeResponse>,
}

pub fn query_trade(
    storage: &dyn Storage,
    trade_id: u64,
) -> Result<TradeInfoResponse, ContractError> {
    let trade_info: TradeInfo = load_trade(storage, trade_id)?;
    Ok(trade_info.try_into()?)
}

pub fn query_counter_trade(
    storage: &dyn Storage,
    trade_id: u64,
    counter_id: u64,
) -> Result<TradeInfoResponse, ContractError> {
    let counter_info: TradeInfo = load_counter_trade(storage, trade_id, counter_id)?;
    Ok(counter_info.try_into()?)
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(deps.storage)
}

// parse trades to human readable format
fn parse_trades(_: &dyn Api, item: StdResult<(u64, TradeInfo)>) -> StdResult<TradeResponse> {
    item.map(|(trade_id, trade)| {
        trade.try_into().map(|trade_info| TradeResponse {
            trade_id,
            counter_id: None,
            trade_info: Some(trade_info),
        })
    })?
}

// parse counter trades to human readable format
fn parse_all_counter_trades(
    _: &dyn Api,
    storage: &dyn Storage,
    item: StdResult<((u64, u64), TradeInfo)>,
) -> StdResult<TradeResponse> {
    item.map(|((trade_id, counter_id), mut counter)| {
        // First two bytes define size [0,8] since we know it's u64 skip it.
        get_actual_counter_state(storage, trade_id, &mut counter)?;
        counter.try_into().map(|trade_info| TradeResponse {
            trade_id,
            counter_id: Some(counter_id),
            trade_info: Some(trade_info),
        })
    })?
}

// parse counter trades to human readable format
fn parse_counter_trades(
    _: &dyn Api,
    storage: &dyn Storage,
    item: StdResult<(u64, TradeInfo)>,
    trade_id: u64,
) -> StdResult<TradeResponse> {
    item.map(|(counter_id, mut counter)| {
        get_actual_counter_state(storage, trade_id, &mut counter)?;
        counter.try_into().map(|trade_info| TradeResponse {
            trade_id,
            counter_id: Some(counter_id),
            trade_info: Some(trade_info),
        })
    })?
}

pub fn trade_filter(
    api: &dyn Api,
    trade_info: &StdResult<TradeResponse>,
    filters: &Option<QueryFilters>,
) -> bool {
    if let Some(filters) = filters {
        let trade = trade_info.as_ref().unwrap();

        (match &filters.states {
            Some(state) => state.contains(&trade.trade_info.as_ref().unwrap().state.to_string()),
            None => true,
        } && match &filters.owner {
            Some(owner) => trade.trade_info.as_ref().unwrap().owner == owner.clone(),
            None => true,
        } && match &filters.has_whitelist {
            Some(has_whitelist) => {
                &trade
                    .trade_info
                    .as_ref()
                    .unwrap()
                    .whitelisted_users
                    .is_empty()
                    != has_whitelist
            }
            None => true,
        } && match &filters.whitelisted_user {
            Some(whitelisted_user) => trade
                .trade_info
                .as_ref()
                .unwrap()
                .whitelisted_users
                .contains(&api.addr_validate(whitelisted_user).unwrap()),
            None => true,
        } && match &filters.wanted_nft {
            Some(wanted_nft) => trade
                .trade_info
                .as_ref()
                .unwrap()
                .additional_info
                .nfts_wanted
                .contains(&api.addr_validate(wanted_nft).unwrap()),
            None => true,
        } && match &filters.contains_token {
            Some(token) => trade
                .trade_info
                .as_ref()
                .unwrap()
                .associated_assets
                .iter()
                .any(|asset| match asset {
                    AssetInfo::Coin(x) => x.denom == token.as_ref(),
                    AssetInfo::Cw20Coin(x) => x.address == token.as_ref(),
                    AssetInfo::Cw721Coin(x) => x.address == token.as_ref(),
                    AssetInfo::Cw1155Coin(x) => x.address == token.as_ref(),
                }),
            None => true,
        } && match &filters.assets_withdrawn {
            Some(assets_withdrawn) => {
                trade.trade_info.clone().unwrap().assets_withdrawn == *assets_withdrawn
            }
            None => true,
        })
    } else {
        true
    }
}

pub fn query_all_trades(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
    filters: Option<QueryFilters>,
) -> StdResult<AllTradesResponse> {
    if let Some(f) = filters.clone() {
        if let Some(counterer) = f.counterer {
            query_all_trades_by_counterer(deps, start_after, limit, counterer, filters)
        } else {
            query_all_trades_raw(deps, start_after, limit, filters)
        }
    } else {
        query_all_trades_raw(deps, start_after, limit, filters)
    }
}

pub fn query_all_trades_raw(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
    filters: Option<QueryFilters>,
) -> StdResult<AllTradesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let mut trades: Vec<TradeResponse> = TRADE_INFO
        .range(deps.storage, None, start.clone(), Order::Descending)
        .take(BASE_LIMIT)
        .map(|kv_item| parse_trades(deps.api, kv_item))
        .filter(|response| trade_filter(deps.api, response, &filters))
        .take(limit)
        .collect::<StdResult<Vec<TradeResponse>>>()?;

    if trades.is_empty() {
        let trade_id = TRADE_INFO
            .keys(deps.storage, None, start, Order::Descending)
            .take(BASE_LIMIT)
            .last();

        if let Some(Ok(trade_id)) = trade_id {
            if trade_id != 0 {
                trades = vec![TradeResponse {
                    trade_id,
                    counter_id: None,
                    trade_info: None,
                }]
            }
        }
    }
    Ok(AllTradesResponse { trades })
}

pub fn query_all_trades_by_counterer(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
    counterer: String,
    filters: Option<QueryFilters>,
) -> StdResult<AllTradesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = start_after.map(|s| Bound::exclusive((s, 0)));

    let counter_filters = Some(QueryFilters {
        owner: Some(counterer),
        ..QueryFilters::default()
    });

    let mut trades: Vec<TradeResponse> = COUNTER_TRADE_INFO
        .range(deps.storage, None, start.clone(), Order::Descending)
        .take(BASE_LIMIT)
        .map(|kv_item| parse_all_counter_trades(deps.api, deps.storage, kv_item))
        .filter(|response| trade_filter(deps.api, response, &counter_filters))
        .filter_map(|response| response.ok())
        // Now we get back the trade_id and query the trade_info
        .map(|response| response.trade_id)
        .unique()
        .map(|trade_id| Ok((trade_id, TRADE_INFO.load(deps.storage, trade_id)?)))
        .map(|kv_item| parse_trades(deps.api, kv_item))
        .filter(|response| trade_filter(deps.api, response, &filters))
        .take(limit)
        .collect::<StdResult<Vec<TradeResponse>>>()?;

    if trades.is_empty() {
        let trade_info: Option<TradeResponse> = COUNTER_TRADE_INFO
            .range(deps.storage, None, start, Order::Descending)
            .take(BASE_LIMIT)
            .map(|kv_item| parse_all_counter_trades(deps.api, deps.storage, kv_item))
            .filter_map(|response| response.ok())
            .map(|response| response.trade_id)
            .unique()
            .map(|trade_id| Ok((trade_id, TRADE_INFO.load(deps.storage, trade_id)?)))
            .filter_map(|kv_item| parse_trades(deps.api, kv_item).ok())
            .last();

        if let Some(trade_info) = trade_info {
            if trade_info.trade_id != 0 || trade_info.counter_id.unwrap() != 0 {
                trades = vec![TradeResponse {
                    trade_id: trade_info.trade_id,
                    counter_id: trade_info.counter_id,
                    trade_info: None,
                }]
            }
        }
    }

    Ok(AllTradesResponse { trades })
}

pub fn query_all_counter_trades(
    deps: Deps,
    start_after: Option<CounterTradeInfo>,
    limit: Option<u32>,
    filters: Option<QueryFilters>,
) -> StdResult<AllCounterTradesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = start_after.map(|s| Bound::exclusive((s.trade_id, s.counter_id)));

    let mut counter_trades: Vec<TradeResponse> = COUNTER_TRADE_INFO
        .range(deps.storage, None, start.clone(), Order::Descending)
        .take(BASE_LIMIT)
        .map(|kv_item| parse_all_counter_trades(deps.api, deps.storage, kv_item))
        .filter(|response| trade_filter(deps.api, response, &filters))
        .take(limit)
        .collect::<StdResult<Vec<TradeResponse>>>()?;

    if counter_trades.is_empty() {
        let id = COUNTER_TRADE_INFO
            .keys(deps.storage, None, start, Order::Descending)
            .take(BASE_LIMIT)
            .last();

        if let Some(Ok((trade_id, counter_id))) = id {
            if trade_id != 0 || counter_id != 0 {
                counter_trades = vec![TradeResponse {
                    trade_id,
                    counter_id: Some(counter_id),
                    trade_info: None,
                }]
            }
        }
    }

    Ok(AllCounterTradesResponse { counter_trades })
}

pub fn query_counter_trades(
    deps: Deps,
    trade_id: u64,
    start_after: Option<u64>,
    limit: Option<u32>,
    filters: Option<QueryFilters>,
) -> StdResult<AllCounterTradesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let mut counter_trades: Vec<TradeResponse> = COUNTER_TRADE_INFO
        .prefix(trade_id)
        .range(deps.storage, None, start.clone(), Order::Descending)
        .take(BASE_LIMIT)
        .map(|kv_item| parse_counter_trades(deps.api, deps.storage, kv_item, trade_id))
        .filter(|response| trade_filter(deps.api, response, &filters))
        .take(limit)
        .collect::<StdResult<Vec<TradeResponse>>>()?;

    if counter_trades.is_empty() {
        let counter_id = COUNTER_TRADE_INFO
            .prefix(trade_id)
            .keys(deps.storage, None, start, Order::Descending)
            .take(BASE_LIMIT)
            .last();

        if let Some(Ok(counter_id)) = counter_id {
            if trade_id != 0 || counter_id != 0 {
                counter_trades = vec![TradeResponse {
                    trade_id,
                    counter_id: Some(counter_id),
                    trade_info: None,
                }]
            }
        }
    }

    Ok(AllCounterTradesResponse { counter_trades })
}
