use cosmwasm_std::{
    to_json_binary, Addr, Decimal, Deps, Env, Order, QueryRequest, StdError, StdResult, WasmQuery,
};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_storage_plus::Bound;

use filters::locality_state_filter;
#[cfg(feature = "sg")]
use sg721_base::QueryMsg as Sg721QueryMsg;

mod filters;

use crate::{
    error::ContractError,
    msg::{
        AllLocalitiesResponse, AllRafflesResponse, ConfigResponse, FeeDiscountResponse,
        LocalityResponse, QueryFilters, RaffleResponse,
    },
    state::{
        get_locality_state, get_raffle_state, load_raffle, LocalityInfo, RaffleInfo, RaffleState,
        CONFIG, LOCALITY_INFO, RAFFLE_INFO, RAFFLE_TICKETS, USER_TICKETS,
    },
    utils::get_raffle_winners,
};

use self::filters::{contains_token_filter, has_gated_rights_filter, owner_filter, state_filter};

// settings for pagination
const MAX_LIMIT: u32 = 100;
const DEFAULT_LIMIT: u32 = 10;
const BASE_LIMIT: usize = 100;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        name: config.name,
        owner: config.owner.to_string(),
        fee_addr: config.fee_addr.to_string(),
        last_raffle_id: config.last_raffle_id.unwrap_or(0),
        minimum_raffle_duration: config.minimum_raffle_duration,
        raffle_fee: config.raffle_fee,
        locks: config.locks,
        nois_proxy_addr: config.nois_proxy_addr.to_string(),
        creation_coins: config.creation_coins,
        nois_proxy_coin: config.nois_proxy_coin,
        max_tickets_per_raffle: config.max_tickets_per_raffle,
        fee_discounts: config.fee_discounts,
    })
}

pub fn query_discount(deps: Deps, user: String) -> StdResult<FeeDiscountResponse> {
    let config = CONFIG.load(deps.storage)?;

    let discounts: Vec<_> = config
        .fee_discounts
        .into_iter()
        .map(|f| {
            (
                f.clone(),
                f.condition.has_advantage(deps, user.clone()).is_ok(),
            )
        })
        .collect();

    let total_discount = Decimal::one()
        - discounts
            .iter()
            .map(|(discount, has_discount)| {
                if *has_discount {
                    Decimal::one() - discount.discount
                } else {
                    Decimal::one()
                }
            })
            .fold(Decimal::one(), |acc, el| acc * el);

    Ok(FeeDiscountResponse {
        discounts,
        total_discount,
    })
}

pub fn query_all_raffles(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
    filters: Option<QueryFilters>,
) -> Result<AllRafflesResponse, ContractError> {
    if filters.is_some() && filters.clone().unwrap().ticket_depositor.is_some() {
        query_all_raffles_by_depositor(deps, env, start_after, limit, filters)
    } else {
        query_all_raffles_raw(deps, env, start_after, limit, filters)
    }
}

/// Query all raffles in which a depositor bought a ticket
/// Returns an empty raffle_info if none where found in the BASE_LIMIT first results
pub fn query_all_raffles_by_depositor(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
    filters: Option<QueryFilters>,
) -> Result<AllRafflesResponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let ticket_depositor = deps.api.addr_validate(
        &filters
            .clone()
            .ok_or_else(|| StdError::generic_err("unauthorized"))?
            .ticket_depositor
            .ok_or_else(|| StdError::generic_err("unauthorized"))?,
    )?;

    let mut raffles = USER_TICKETS
        .prefix(&ticket_depositor)
        .keys(deps.storage, None, start.clone(), Order::Descending)
        .take(BASE_LIMIT)
        .filter_map(|response| response.ok())
        .map(|raffle_id| Ok((raffle_id, load_raffle(deps.storage, raffle_id).unwrap()))) // This unwrap is safe if the data structure was respected
        .filter(|response| match response {
            Ok((_id, raffle_info)) => raffle_filter(deps, env.clone(), raffle_info, &filters),
            Err(_) => false,
        })
        .map(|kv_item| parse_raffles(deps, &env, kv_item))
        .take(limit)
        .collect::<Result<Vec<RaffleResponse>, ContractError>>()?;

    if raffles.is_empty() {
        let last_raffle_id = USER_TICKETS
            .prefix(&ticket_depositor)
            .keys(deps.storage, None, start.clone(), Order::Descending)
            .take(BASE_LIMIT)
            .filter_map(|response| response.ok())
            .last();
        if let Some(raffle_id) = last_raffle_id {
            if raffle_id != 0 {
                raffles = vec![RaffleResponse {
                    raffle_id,
                    raffle_state: RaffleState::Claimed,
                    raffle_info: None,
                }]
            }
        }
    }

    Ok(AllRafflesResponse { raffles })
}

// parse raffles to human readable format
fn parse_raffles(
    deps: Deps,
    env: &Env,
    item: StdResult<(u64, RaffleInfo)>,
) -> Result<RaffleResponse, ContractError> {
    item.map_err(Into::into)
        .and_then(|(raffle_id, mut raffle)| {
            let raffle_state = get_raffle_state(env, &raffle);
            add_raffle_winners(deps, env, raffle_id, &mut raffle)?;
            Ok(RaffleResponse {
                raffle_id,
                raffle_state,
                raffle_info: Some(raffle),
            })
        })
}

/// Query all ticket onwers within a raffle
///
pub fn query_all_tickets(
    deps: Deps,
    _env: Env,
    raffle_id: u64,
    start_after: Option<u32>,
    limit: Option<u32>,
) -> StdResult<Vec<String>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    RAFFLE_TICKETS
        .prefix(raffle_id)
        .range(deps.storage, start.clone(), None, Order::Ascending)
        .map(|kv_item| Ok(kv_item?.1.to_string()))
        .take(limit)
        .collect()
}

pub fn query_all_raffles_raw(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
    filters: Option<QueryFilters>,
) -> Result<AllRafflesResponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let mut raffles: Vec<RaffleResponse> = RAFFLE_INFO
        .range(deps.storage, None, start.clone(), Order::Descending)
        .take(BASE_LIMIT)
        .filter(|response| match response {
            Ok((_id, raffle_info)) => raffle_filter(deps, env.clone(), raffle_info, &filters),
            Err(_) => false,
        })
        .map(|kv_item| parse_raffles(deps, &env, kv_item))
        .take(limit)
        .collect::<Result<Vec<RaffleResponse>, ContractError>>()?;

    if raffles.is_empty() {
        let raffle_id = RAFFLE_INFO
            .keys(deps.storage, None, start, Order::Descending)
            .take(BASE_LIMIT)
            .last();

        if let Some(Ok(raffle_id)) = raffle_id {
            if raffle_id != 0 {
                raffles = vec![RaffleResponse {
                    raffle_id,
                    raffle_state: RaffleState::Claimed,
                    raffle_info: None,
                }]
            }
        }
    }
    Ok(AllRafflesResponse { raffles })
}

pub fn query_all_localities_raw(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
    filters: Option<QueryFilters>,
) -> Result<AllLocalitiesResponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let mut localities: Vec<LocalityResponse> = LOCALITY_INFO
        .range(deps.storage, None, start.clone(), Order::Descending)
        .take(BASE_LIMIT)
        .filter(|response| match response {
            Ok((_id, raffle_info)) => locality_filter(deps, env.clone(), raffle_info, &filters),
            Err(_) => false,
        })
        .map(|kv_item| parse_localities(deps, &env, kv_item))
        .take(limit)
        .collect::<Result<Vec<LocalityResponse>, ContractError>>()?;
    Ok(AllLocalitiesResponse { localities })
}

// parse localities to human readable format
fn parse_localities(
    deps: Deps,
    env: &Env,
    item: StdResult<(u64, LocalityInfo)>,
) -> Result<LocalityResponse, ContractError> {
    item.map_err(Into::into).and_then(|(id, mut info)| {
        let state = get_locality_state(env, &info.clone());
        // add_raffle_winners(deps, env, raffle_id, &mut raffle)?;
        Ok(LocalityResponse {
            id,
            state,
            info: info.clone(),
            frequency: info.frequency,
        })
    })
}

pub fn raffle_filter(
    deps: Deps,
    env: Env,
    raffle_info: &RaffleInfo,
    filters: &Option<QueryFilters>,
) -> bool {
    if let Some(filters) = filters {
        state_filter(&env, raffle_info, filters)
            && owner_filter(raffle_info, filters)
            && contains_token_filter(raffle_info, filters)
            && has_gated_rights_filter(deps, raffle_info, filters)
    } else {
        true
    }
}

pub fn locality_filter(
    deps: Deps,
    env: Env,
    locality_info: &LocalityInfo,
    filters: &Option<QueryFilters>,
) -> bool {
    if let Some(filters) = filters {
        locality_state_filter(&env, locality_info, filters)
    } else {
        true
    }
}

pub fn is_nft_owner(
    deps: Deps,
    sender: Addr,
    nft_address: String,
    token_id: String,
) -> Result<(), StdError> {
    let owner_response: OwnerOfResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: nft_address,
            msg: to_json_binary(&Cw721QueryMsg::OwnerOf {
                token_id,
                include_expired: None,
            })?,
        }))?;

    if owner_response.owner != sender {
        return Err(StdError::generic_err(
            "message sender is not owner of tokens being raffled",
        ));
    }
    Ok(())
}

#[cfg(feature = "sg")]
pub fn is_sg721_owner(
    deps: Deps,
    sender: Addr,
    nft_address: String,
    token_id: String,
) -> Result<(), StdError> {
    let owner_response: OwnerOfResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: nft_address,
            msg: to_json_binary(&Sg721QueryMsg::OwnerOf {
                token_id,
                include_expired: None,
            })?,
        }))?;

    if owner_response.owner != sender {
        return Err(StdError::generic_err(
            "message sender is not owner of tokens being raffled",
        ));
    }
    Ok(())
}

/// Query the number of tickets a ticket_depositor bought in a specific raffle, designated by a raffle_id
pub fn query_ticket_count(
    deps: Deps,
    _env: Env,
    raffle_id: u64,
    ticket_depositor: String,
) -> StdResult<u32> {
    USER_TICKETS.load(
        deps.storage,
        (&deps.api.addr_validate(&ticket_depositor)?, raffle_id),
    )
}

pub fn add_raffle_winners(
    deps: Deps,
    env: &Env,
    raffle_id: u64,
    raffle_info: &mut RaffleInfo,
) -> Result<(), ContractError> {
    if raffle_info.randomness.is_some() {
        if raffle_info.number_of_tickets == 0u32
            || raffle_info.number_of_tickets
                < raffle_info.raffle_options.min_ticket_number.unwrap_or(0)
        {
            raffle_info.winners = vec![raffle_info.owner.clone()];
        } else {
            // We calculate the winner of the raffle
            raffle_info.winners = get_raffle_winners(deps, env, raffle_id, raffle_info.clone())?;
        };
    }

    Ok(())
}
