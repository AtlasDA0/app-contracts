use cosmwasm_std::{
    entry_point, to_json_binary, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, QueryResponse,
    StdResult,
};
use sg_std::StargazeMsgWrapper;

use crate::error::ContractError;
use crate::execute::{
    execute_buy_tickets, execute_cancel_raffle, execute_create_raffle, execute_determine_winner,
    execute_modify_raffle, execute_receive, execute_receive_nois, execute_toggle_lock,
    execute_update_config, execute_update_randomness,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, RaffleResponse};
use crate::query::{query_all_raffles, query_all_tickets, query_config, query_ticket_count};
use crate::state::{
    get_raffle_state, load_raffle, Config, CONFIG, STATIC_RAFFLE_CREATION_FEE,
    MINIMUM_RAFFLE_DURATION, MINIMUM_RAFFLE_TIMEOUT,
};
use cw2::set_contract_version;

pub type Response = cosmwasm_std::Response<StargazeMsgWrapper>;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let nois_proxy_addr = deps
        .api
        .addr_validate(&msg.nois_proxy_addr)
        .map_err(|_| ContractError::InvalidProxyAddress)?;

    let creation_fee_amount = match msg.creation_fee_amount {
        Some(int) => int,
        None => STATIC_RAFFLE_CREATION_FEE.into(),
    };

    let creation_fee_denom = match msg.creation_fee_denom {
        Some(cfd) => cfd,
        None => vec!["ustars".to_string(), "usstars".to_string()],
    };

    let config = Config {
        name: msg.name,
        owner: deps
            .api
            .addr_validate(&msg.owner.unwrap_or_else(|| info.sender.to_string()))?,
        fee_addr: deps
            .api
            .addr_validate(&msg.fee_addr.unwrap_or_else(|| info.sender.to_string()))?,
        last_raffle_id: None,
        minimum_raffle_duration: msg
            .minimum_raffle_duration
            .unwrap_or(MINIMUM_RAFFLE_DURATION)
            .max(MINIMUM_RAFFLE_DURATION),
        minimum_raffle_timeout: msg
            .minimum_raffle_timeout
            .unwrap_or(MINIMUM_RAFFLE_TIMEOUT)
            .max(MINIMUM_RAFFLE_TIMEOUT),
        raffle_fee: msg.raffle_fee.unwrap_or(Decimal::zero()),
        creation_fee_denom,
        creation_fee_amount,
        lock: false,
        nois_proxy_addr,
        nois_proxy_denom: msg.nois_proxy_denom,
        nois_proxy_amount: msg.nois_proxy_amount,
    };

    config.validate_fee()?;

    CONFIG.save(deps.storage, &config)?;
    set_contract_version(
        deps.storage,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
    set_contract_version(
        deps.storage,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            name,
            owner,
            fee_addr,
            minimum_raffle_duration,
            minimum_raffle_timeout,
            creation_fee_denom,
            creation_fee_amount,
            raffle_fee,
            nois_proxy_addr,
            nois_proxy_denom,
            nois_proxy_amount,
        } => execute_update_config(
            deps,
            env,
            info,
            name,
            owner,
            fee_addr,
            minimum_raffle_duration,
            minimum_raffle_timeout,
            creation_fee_denom,
            creation_fee_amount,
            raffle_fee,
            nois_proxy_addr,
            nois_proxy_denom,
            nois_proxy_amount,
        ),
        ExecuteMsg::CreateRaffle {
            owner,
            assets,
            raffle_options,
            raffle_ticket_price,
        } => execute_create_raffle(
            deps,
            env,
            info,
            owner,
            assets,
            raffle_ticket_price,
            raffle_options,
        ),
        ExecuteMsg::CancelRaffle { raffle_id } => execute_cancel_raffle(deps, env, info, raffle_id),
        ExecuteMsg::ModifyRaffle {
            raffle_id,
            raffle_ticket_price,
            raffle_options,
        } => execute_modify_raffle(
            deps,
            env,
            info,
            raffle_id,
            raffle_ticket_price,
            raffle_options,
        ),
        ExecuteMsg::BuyTicket {
            raffle_id,
            ticket_count,
            sent_assets,
        } => execute_buy_tickets(deps, env, info, raffle_id, ticket_count, sent_assets),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::DetermineWinner { raffle_id } => {
            execute_determine_winner(deps, env, info, raffle_id)
        }
        ExecuteMsg::UpdateRandomness { raffle_id } => {
            execute_update_randomness(deps, env, info, raffle_id)
        }
        ExecuteMsg::NoisReceive { callback } => execute_receive_nois(deps, env, info, callback),
        // Admin messages
        ExecuteMsg::ToggleLock { lock } => execute_toggle_lock(deps, env, info, lock),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?)?,
        QueryMsg::RaffleInfo { raffle_id } => {
            let raffle_info = load_raffle(deps.storage, raffle_id)?;
            to_json_binary(&RaffleResponse {
                raffle_id,
                raffle_state: get_raffle_state(env, raffle_info.clone()),
                raffle_info: Some(raffle_info),
            })?
        }
        QueryMsg::AllRaffles {
            start_after,
            limit,
            filters,
        } => to_json_binary(&query_all_raffles(deps, env, start_after, limit, filters)?)?,
        QueryMsg::AllTickets {
            raffle_id,
            start_after,
            limit,
        } => to_json_binary(&query_all_tickets(
            deps,
            env,
            raffle_id,
            start_after,
            limit,
        )?)?,
        QueryMsg::TicketCount { owner, raffle_id } => {
            to_json_binary(&query_ticket_count(deps, env, raffle_id, owner)?)?
        }
    };
    Ok(response)
}
