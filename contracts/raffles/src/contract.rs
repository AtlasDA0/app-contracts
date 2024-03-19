use cosmwasm_std::{
    coin, ensure, ensure_eq, entry_point, to_json_binary, Decimal, Deps, DepsMut, Empty, Env,
    MessageInfo, QueryResponse, StdResult, Uint128,
};

use crate::{
    error::ContractError,
    execute::{
        execute_buy_tickets, execute_cancel_raffle, execute_create_raffle, execute_modify_raffle,
        execute_receive, execute_receive_nois, execute_sudo_toggle_lock, execute_toggle_lock,
        execute_update_config,
    },
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, RaffleResponse},
    query::{query_all_raffles, query_all_tickets, query_config, query_ticket_count},
    state::{
        get_raffle_state, load_raffle, Config, CONFIG, MAX_TICKET_NUMBER, MINIMUM_RAFFLE_DURATION,
        STATIC_RAFFLE_CREATION_FEE,
    },
    utils::get_nois_randomness,
};
use utils::{
    state::{is_valid_name, Locks, SudoMsg, NATIVE_DENOM},
    types::Response,
};

use cw2::set_contract_version;

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

    // define the accepted fee coins
    let creation_coins = match msg.creation_coins {
        Some(cc_msg) => cc_msg,
        None => vec![coin(STATIC_RAFFLE_CREATION_FEE, NATIVE_DENOM)], // TODO: update to handle ibc contract support native denoms
    };

    // fee decimal range
    ensure!(
        msg.raffle_fee >= Decimal::zero() && msg.raffle_fee <= Decimal::one(),
        ContractError::InvalidFeeRate {}
    );
    ensure!(
        msg.nois_proxy_coin.amount >= Uint128::zero(),
        ContractError::InvalidProxyCoin
    );
    // valid name
    if !is_valid_name(&msg.name) {
        return Err(ContractError::InvalidName {});
    }
    // define internal contract
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
        raffle_fee: msg.raffle_fee,
        locks: Locks {
            lock: false,
            sudo_lock: false,
        },
        nois_proxy_addr,
        nois_proxy_coin: msg.nois_proxy_coin,
        creation_coins,
        max_tickets_per_raffle: Some(msg.max_ticket_number.unwrap_or(MAX_TICKET_NUMBER)),
    };

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
        ExecuteMsg::NoisReceive { callback } => execute_receive_nois(deps, env, info, callback),
        ExecuteMsg::ToggleLock { lock } => execute_toggle_lock(deps, env, info, lock),
        ExecuteMsg::UpdateConfig {
            name,
            owner,
            fee_addr,
            minimum_raffle_duration,
            max_tickets_per_raffle,
            raffle_fee,
            nois_proxy_addr,
            nois_proxy_coin,
            creation_coins,
        } => execute_update_config(
            deps,
            env,
            info,
            name,
            owner,
            fee_addr,
            minimum_raffle_duration,
            max_tickets_per_raffle,
            raffle_fee,
            nois_proxy_addr,
            nois_proxy_coin,
            creation_coins,
        ),
        ExecuteMsg::UpdateRandomness { raffle_id } => {
            let config = CONFIG.load(deps.storage)?;
            ensure_eq!(info.sender, config.owner, ContractError::Unauthorized);
            let msg = get_nois_randomness(deps.as_ref(), raffle_id)?;
            Ok(Response::new().add_message(msg))
        }
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

// sudo entry point for governance override
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::ToggleLock { lock } => {
            execute_sudo_toggle_lock(deps, env, lock).map_err(|_| ContractError::ContractBug {})
        }
    }
}
