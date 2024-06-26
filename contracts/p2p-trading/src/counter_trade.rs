use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response};

use p2p_trading_export::state::{AdditionalTradeInfo, AssetInfo, TradeInfo, TradeState};

use crate::error::ContractError;
use crate::messages::set_comment;
use crate::state::{
    add_cw1155_coin, add_cw20_coin, add_cw721_coin, add_funds, can_suggest_counter_trade,
    is_counter_trader, load_trade, COUNTER_TRADE_INFO, LAST_USER_COUNTER_TRADE, TRADE_INFO,
};
use crate::trade::{
    _are_assets_in_trade, _create_receive_asset_messages, _create_withdraw_messages_unsafe,
    _try_withdraw_assets_unsafe, check_and_create_withdraw_messages,
};

/// Query the last counter_trade created by the owner for the `trade_id`
/// This should only be used in the same transaction as the counter_trade creation.
/// Otherwise, specify the counter_id directly in the transaction and this is not needed
pub fn get_last_counter_id_created(
    deps: Deps,
    by: String,
    trade_id: u64,
) -> Result<u64, ContractError> {
    let owner = deps.api.addr_validate(&by)?;
    LAST_USER_COUNTER_TRADE
        .load(deps.storage, (&owner, trade_id))
        .map_err(|_| ContractError::NotFoundInCounterTradeInfo {})
}

/// Create a new counter_trade and assign it a unique id for the specified `trade_id`
/// Saves this counter_trade as the last one in the trade created by the user
pub fn suggest_counter_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
    comment: Option<String>,
) -> Result<Response, ContractError> {
    // We start by verifying it is possible to suggest a counter trade to that trade
    // It also checks if the trade exists
    // And that the sender is whitelisted (in case the trade is private)
    let mut trade_info = can_suggest_counter_trade(deps.storage, trade_id, &info.sender)?;

    // We start by creating a new trade_id (simply incremented from the last id)
    trade_info.last_counter_id = trade_info
        .last_counter_id
        .map_or(Some(0), |id| Some(id + 1));
    if trade_info.state == TradeState::Published {
        trade_info.state = TradeState::Countered;
    }
    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    let counter_id = trade_info.last_counter_id.unwrap(); // This is safe, as we just created a ast_counter_id` if it didn't exist.

    COUNTER_TRADE_INFO.update(
        deps.storage,
        (trade_id, counter_id),
        |counter| match counter {
            // If the trade id already exists, the contract is faulty
            // Or an external error happened, or whatever...
            // In that case, we emit an error
            // The priority is : We do not want to overwrite existing data
            Some(_) => Err(ContractError::ExistsInCounterTradeInfo {}),
            None => Ok(TradeInfo {
                owner: info.sender.clone(),
                additional_info: AdditionalTradeInfo {
                    time: env.block.time,
                    ..Default::default()
                },
                ..Default::default()
            }),
        },
    )?;

    // We also set the last trade_id created to this id
    LAST_USER_COUNTER_TRADE.save(deps.storage, (&info.sender, trade_id), &counter_id)?;

    // And the eventual comment sent along with the transaction
    if let Some(comment) = comment {
        set_comment(deps, env, info.clone(), trade_id, Some(counter_id), comment)?;
    }

    Ok(Response::new()
        .add_attribute("action", "create_counter_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", info.sender))
}

pub fn counter_id_or_last(
    deps: Deps,
    trader: Addr,
    trade_id: u64,
    counter_id: Option<u64>,
) -> Result<u64, ContractError> {
    match counter_id {
        Some(counter_id) => Ok(counter_id),
        None => get_last_counter_id_created(deps, trader.to_string(), trade_id),
    }
}

/// We prepare the info before asset addition
/// 1. If the trade_id is not specified, we get the last trade_id created by the sender
/// 2. We verify the trade can be modified
pub fn prepare_counter_modification(
    deps: Deps,
    trader: Addr,
    trade_id: u64,
    counter_id: Option<u64>,
) -> Result<(u64, TradeInfo), ContractError> {
    let counter_id = counter_id_or_last(deps, trader.clone(), trade_id, counter_id)?;

    let counter_info = is_counter_trader(deps.storage, &trader, trade_id, counter_id)?;

    if counter_info.state != TradeState::Created {
        return Err(ContractError::WrongTradeState {
            state: counter_info.state,
        });
    }
    Ok((counter_id, counter_info))
}

pub fn add_asset_to_counter_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: Option<u64>,
    asset: AssetInfo,
) -> Result<Response, ContractError> {
    let (counter_id, _) =
        prepare_counter_modification(deps.as_ref(), info.sender.clone(), trade_id, counter_id)?;

    match asset.clone() {
        AssetInfo::Coin(coin) => COUNTER_TRADE_INFO.update(
            deps.storage,
            (trade_id, counter_id),
            add_funds(coin, info.funds.clone()),
        ),
        AssetInfo::Cw20Coin(token) => COUNTER_TRADE_INFO.update(
            deps.storage,
            (trade_id, counter_id),
            add_cw20_coin(token.address.clone(), token.amount),
        ),
        AssetInfo::Cw721Coin(token) => COUNTER_TRADE_INFO.update(
            deps.storage,
            (trade_id, counter_id),
            add_cw721_coin(token.address.clone(), token.token_id),
        ),
        AssetInfo::Cw1155Coin(token) => COUNTER_TRADE_INFO.update(
            deps.storage,
            (trade_id, counter_id),
            add_cw1155_coin(token.address.clone(), token.token_id.clone(), token.value),
        ),
    }?;

    // We load the trade_info for events
    let trade_info = load_trade(deps.storage, trade_id)?;

    // Now we need to transfer the token
    Ok(_create_receive_asset_messages(env, info.clone(), asset)?
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", info.sender))
}

/// Allows to withdraw assets while creating a counter_trade, Refer to the `trade.rs`file for more information (similar mechanism)
pub fn withdraw_counter_trade_assets_while_creating(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: u64,
    assets: Vec<(u16, AssetInfo)>,
) -> Result<Response, ContractError> {
    let mut counter_info = is_counter_trader(deps.storage, &info.sender, trade_id, counter_id)?;
    if counter_info.state != TradeState::Created {
        return Err(ContractError::CounterTradeAlreadyPublished {});
    }
    _are_assets_in_trade(&counter_info, &assets)?;

    _try_withdraw_assets_unsafe(&mut counter_info, &assets)?;

    // We make sure the asset was not the advertised asset
    // For CW721, we match the whole assetInfo
    // For Cw1155 we only match the address and the token_id
    if let Some(preview) = counter_info.additional_info.trade_preview.clone() {
        match preview {
            AssetInfo::Cw721Coin(_) => {
                if assets.iter().any(|r| r.1 == preview) {
                    counter_info.additional_info.trade_preview = None;
                }
            }
            AssetInfo::Cw1155Coin(preview_coin) => {
                if assets.iter().any(|r| match r.1.clone() {
                    AssetInfo::Cw1155Coin(coin) => {
                        coin.address == preview_coin.address
                            && coin.token_id == preview_coin.token_id
                    }
                    _ => false,
                }) {
                    counter_info.additional_info.trade_preview = None;
                }
            }
            _ => {}
        }
    }

    COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &counter_info)?;

    let res = _create_withdraw_messages_unsafe(
        &env.contract.address,
        &info.sender,
        &assets.iter().map(|x| x.1.clone()).collect(),
    )?;
    // We load the trade_info for events
    let trade_info = load_trade(deps.storage, trade_id)?;
    Ok(res
        .add_attribute("action", "remove_from_counter_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", info.sender))
}

/// Confirm (and publish) a counter_trade when creation is finished
pub fn confirm_counter_trade(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: Option<u64>,
) -> Result<Response, ContractError> {
    // We check the counter exists and belongs to the sender
    let counter_id = counter_id_or_last(deps.as_ref(), info.sender.clone(), trade_id, counter_id)?;
    let mut counter_info = is_counter_trader(deps.storage, &info.sender, trade_id, counter_id)?;

    // We check the counter can be confirmed
    if counter_info.state != TradeState::Created {
        return Err(ContractError::CantChangeCounterTradeState {
            from: counter_info.state,
            to: TradeState::Published,
        });
    }
    // We confirm the counter_trade
    counter_info.state = TradeState::Published;
    COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &counter_info)?;

    // We load the trade_info for events
    let trade_info = load_trade(deps.storage, trade_id)?;

    Ok(Response::new()
        .add_attribute("action", "confirm_counter_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", info.sender))
}

/// Cancel a counter_trade
/// The counter_trade isn't modifiable, but the funds are withdrawable after this call.
pub fn cancel_counter_trade(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: u64,
) -> Result<Response, ContractError> {
    // Only the initial trader can cancel the trade
    let mut counter_info = is_counter_trader(deps.storage, &info.sender, trade_id, counter_id)?;

    // We can't cancel an accepted counter_trade
    if counter_info.state == TradeState::Accepted {
        return Err(ContractError::CantChangeCounterTradeState {
            from: counter_info.state,
            to: TradeState::Cancelled,
        });
    }
    counter_info.state = TradeState::Cancelled;

    // We store the new trade status
    COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &counter_info)?;

    // We load the trade_info for events
    let trade_info = load_trade(deps.storage, trade_id)?;

    Ok(Response::new()
        .add_attribute("action", "cancel_counter_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", info.sender))
}

/// Withdraw all assets from a created (not published yet), refused or cancelled counter_trade
/// If the counter_trade is only in the created state, it is automatically cancelled before withdrawing assets
pub fn withdraw_all_from_counter(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: u64,
) -> Result<Response, ContractError> {
    let mut counter_info = is_counter_trader(deps.storage, &info.sender, trade_id, counter_id)?;

    // If the counter is still in the created state, we cancel it
    if counter_info.state == TradeState::Created {
        counter_info.state = TradeState::Cancelled;
    }

    // This fuction call is possible only if the counter was refused or if this counter was cancelled
    if !(counter_info.state == TradeState::Refused || counter_info.state == TradeState::Cancelled) {
        return Err(ContractError::CounterTradeNotAborted {});
    }

    // We create withdraw messages to send the funds back to the counter trader
    let res = check_and_create_withdraw_messages(env, &info.sender, &counter_info)?;
    counter_info.assets_withdrawn = true;
    COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &counter_info)?;

    // We load the trade_info for events
    let trade_info = load_trade(deps.storage, trade_id)?;

    Ok(res
        .add_attribute("action", "withdraw_all_funds")
        .add_attribute("type", "counter_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", counter_info.owner))
}
