use std::convert::TryInto;

use cosmwasm_std::{Coins, DepsMut, Env, MessageInfo, Response};

use crate::{
    counter_trade::{add_asset_to_counter_trade, confirm_counter_trade, suggest_counter_trade},
    state::{can_suggest_counter_trade, LAST_USER_COUNTER_TRADE},
    trade::accept_trade,
    ContractError,
};

pub fn direct_buy(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
) -> Result<Response, ContractError> {
    // We make sure the sender can buy the specified assets
    let trade_info = can_suggest_counter_trade(deps.storage, trade_id, &info.sender)?;

    // We make sure the necessary funds are sent with this message
    {
        let mut all_tokens_wanted: Coins = trade_info.additional_info.tokens_wanted.try_into()?;
        if all_tokens_wanted.is_empty() {
            return Err(ContractError::NotBuyableDirectly {});
        }

        for token in info.funds.clone() {
            all_tokens_wanted.sub(token)?;
        }

        // If all subtraction were successful and all_tokens_wanted is empty, the paiement is full
        if !all_tokens_wanted.is_empty() {
            return Err(ContractError::NotEnoughPaiement {
                missing_funds: all_tokens_wanted.into(),
            });
        }
    }

    // The buy is legitimate, we exchange the assets and close the trade
    let all_responses = {
        let buyer_info = MessageInfo {
            sender: info.sender.clone(),
            funds: vec![],
        };
        let trader_info = MessageInfo {
            sender: trade_info.owner,
            funds: vec![],
        };
        let suggest_counter_trade_response = suggest_counter_trade(
            deps.branch(),
            env.clone(),
            buyer_info.clone(),
            trade_id,
            Some("Direct Buy Offer".to_string()),
        )?;
        let counter_id = LAST_USER_COUNTER_TRADE.load(deps.storage, (&info.sender, trade_id))?;

        let all_add_assets_response = info
            .funds
            .into_iter()
            .map(|f| {
                let mut info = buyer_info.clone();
                info.funds = vec![f.clone()];
                add_asset_to_counter_trade(
                    deps.branch(),
                    env.clone(),
                    info.clone(),
                    trade_id,
                    Some(counter_id),
                    utils::state::AssetInfo::Coin(f),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let publish_counter_trade_response =
            confirm_counter_trade(deps.branch(), env.clone(), buyer_info, trade_id, None)?;

        // We accept the trade on behalf of the owner
        let accept_response = accept_trade(
            deps.branch(),
            env.clone(),
            trader_info,
            trade_id,
            counter_id,
            Some("automatically accepted offer".to_string()),
        )?;

        [
            vec![
                suggest_counter_trade_response,
                publish_counter_trade_response,
            ],
            all_add_assets_response,
            vec![accept_response],
        ]
        .concat()
    };

    let mut response = Response::new();

    for individual_response in all_responses {
        response = response.add_submessages(individual_response.messages);
        response = response.add_attributes(individual_response.attributes);
        response = response.add_events(individual_response.events);
    }

    Ok(response)
}
