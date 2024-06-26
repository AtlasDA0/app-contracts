use crate::counter_trade::prepare_counter_modification;
use crate::error::ContractError;
use crate::state::{
    is_counter_trader, is_trader, load_counter_trade, COUNTER_TRADE_INFO, TRADE_INFO,
};
use crate::trade::prepare_trade_modification;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use p2p_trading_export::msg::AddAssetAction;
use p2p_trading_export::state::{AssetInfo, Comment, TradeState};

pub fn review_counter_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: u64,
    comment: Option<String>,
) -> Result<Response, ContractError> {
    // Only the initial trader can cancel the trade !
    let trade_info = is_trader(deps.storage, &info.sender, trade_id)?;

    // We check the counter trade exists !
    let mut counter_info = load_counter_trade(deps.storage, trade_id, counter_id)?;

    if trade_info.state == TradeState::Accepted {
        return Err(ContractError::TradeAlreadyAccepted {});
    }
    if trade_info.state == TradeState::Cancelled {
        return Err(ContractError::TradeCancelled {});
    }

    // Only a published counter trade can be reviewed
    if counter_info.state != TradeState::Published {
        return Err(ContractError::CantChangeCounterTradeState {
            from: counter_info.state,
            to: TradeState::Created,
        });
    }

    counter_info.state = TradeState::Created;
    counter_info.additional_info.trader_comment = comment.map(|comment| Comment {
        time: env.block.time,
        comment,
    });

    // Then we need to change the trade status that we may have changed
    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;
    COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &counter_info)?;

    Ok(Response::new()
        .add_attribute("action", "review_counter_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", counter_info.owner))
}

pub fn set_comment(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: Option<u64>,
    comment: String,
) -> Result<Response, ContractError> {
    let comment = Comment {
        time: env.block.time,
        comment,
    };

    if let Some(counter_id) = counter_id {
        let mut counter_info = is_counter_trader(deps.storage, &info.sender, trade_id, counter_id)?;
        counter_info.additional_info.owner_comment = Some(comment);
        COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &counter_info)?;
    } else {
        let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;
        trade_info.additional_info.owner_comment = Some(comment);
        TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;
    }
    let partial_res = Response::new()
        .add_attribute("action", "set_comment")
        .add_attribute("trade_id", trade_id.to_string());

    if let Some(counter_id) = counter_id {
        Ok(partial_res.add_attribute("counter_id", counter_id.to_string()))
    } else {
        Ok(partial_res)
    }
}

pub fn set_trade_preview(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    action: AddAssetAction,
    asset: AssetInfo,
) -> Result<Response, ContractError> {
    // First we get the trade_info to modify
    let (trade_id, counter_id, mut trade_info) = match action {
        AddAssetAction::ToLastTrade {} => {
            let (trade_id, trade_info) =
                prepare_trade_modification(deps.as_ref(), info.sender, None)?;
            (trade_id, None, trade_info)
        }
        AddAssetAction::ToTrade { trade_id } => {
            let (trade_id, trade_info) =
                prepare_trade_modification(deps.as_ref(), info.sender, Some(trade_id))?;
            (trade_id, None, trade_info)
        }
        AddAssetAction::ToLastCounterTrade { trade_id } => {
            let (counter_id, trade_info) =
                prepare_counter_modification(deps.as_ref(), info.sender, trade_id, None)?;
            (trade_id, Some(counter_id), trade_info)
        }
        AddAssetAction::ToCounterTrade {
            trade_id,
            counter_id,
        } => {
            let (counter_id, trade_info) = prepare_counter_modification(
                deps.as_ref(),
                info.sender,
                trade_id,
                Some(counter_id),
            )?;
            (trade_id, Some(counter_id), trade_info)
        }
    };

    // Then we verify we can set the asset as preview

    if !trade_info.associated_assets.iter().any(|r| *r == asset) {
        return Err(ContractError::AssetNotInTrade {});
    }

    // And set it
    trade_info.additional_info.trade_preview = Some(asset);

    // Finally we save the trade info behind ourselves
    match counter_id {
        Some(counter_id) => {
            COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &trade_info)?
        }
        None => TRADE_INFO.save(deps.storage, trade_id, &trade_info)?,
    }

    let mut res = Response::new()
        .add_attribute("action", "set_trade_preview")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", trade_info.owner);

    if let Some(counter_id) = counter_id {
        res = res.add_attribute("counter_id", counter_id.to_string())
    }

    Ok(res)
}
