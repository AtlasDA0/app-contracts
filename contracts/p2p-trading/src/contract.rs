#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};
use cosmwasm_std::{BankMsg, Coin};
use cw2::set_contract_version;
use utils::payment::assert_payment;
use utils::state::AssetInfo;

use crate::error::ContractError;

use crate::state::{
    is_owner, load_counter_trade, load_trade, CONTRACT_INFO, COUNTER_TRADE_INFO, TRADE_INFO,
};
use p2p_trading_export::msg::{AddAssetAction, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use p2p_trading_export::state::{ContractInfo, TradeState};

use crate::counter_trade::{
    add_asset_to_counter_trade, cancel_counter_trade, confirm_counter_trade, suggest_counter_trade,
    withdraw_all_from_counter, withdraw_counter_trade_assets_while_creating,
};
use crate::trade::{
    accept_trade, add_asset_to_trade, add_nfts_wanted, add_tokens_wanted, add_whitelisted_users,
    cancel_trade, check_and_create_withdraw_messages, confirm_trade, create_trade,
    flush_nfts_wanted, flush_tokens_wanted, gather_royalties, refuse_counter_trade,
    remove_nfts_wanted, remove_tokens_wanted, remove_whitelisted_users, set_nfts_wanted,
    set_tokens_wanted, withdraw_all_from_trade, withdraw_trade_assets_while_creating,
};

use crate::messages::{review_counter_trade, set_comment, set_trade_preview};
use crate::query::{
    query_all_counter_trades, query_all_trades, query_contract_info, query_counter_trade,
    query_counter_trades, query_trade,
};

const CONTRACT_NAME: &str = "illiquidly.io:p2p-trading";
const CONTRACT_VERSION: &str = "0.1.0";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Verify the contract name

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    msg.validate()?;

    if msg.accept_trade_fee.iter().any(|c| c.amount.is_zero()) {
        return Err(StdError::generic_err("Fee can't be zero").into());
    }

    // store token info
    let data = ContractInfo {
        name: msg.name,
        owner: deps
            .api
            .addr_validate(&msg.owner.unwrap_or_else(|| info.sender.to_string()))?,
        last_trade_id: None,
        accept_trade_fee: msg.accept_trade_fee,
        treasury: deps.api.addr_validate(&msg.treasury)?,
        fund_fee: msg.fund_fee,
    };
    CONTRACT_INFO.save(deps.storage, &data)?;
    Ok(Response::default()
        .add_attribute("action", "init")
        .add_attribute("contract", "p2p-trading")
        .add_attribute("owner", data.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateTrade {
            whitelisted_users,
            comment,
        } => create_trade(deps, env, info, whitelisted_users, comment),
        ExecuteMsg::AddAsset { action, asset } => add_asset(deps, env, info, action, asset),
        ExecuteMsg::RemoveAssets {
            trade_id,
            counter_id,
            assets,
        } => withdraw_assets_while_creating(deps, env, info, trade_id, counter_id, assets),
        ExecuteMsg::AddWhitelistedUsers {
            trade_id,
            whitelisted_users,
        } => add_whitelisted_users(
            deps.storage,
            deps.api,
            env,
            info,
            trade_id,
            whitelisted_users,
        ),
        ExecuteMsg::RemoveWhitelistedUsers {
            trade_id,
            whitelisted_users,
        } => remove_whitelisted_users(deps, env, info, trade_id, whitelisted_users),
        ExecuteMsg::AddNFTsWanted {
            trade_id,
            nfts_wanted,
        } => add_nfts_wanted(deps, env, info, trade_id, nfts_wanted),
        ExecuteMsg::RemoveNFTsWanted {
            trade_id,
            nfts_wanted,
        } => remove_nfts_wanted(deps, env, info, trade_id, nfts_wanted),
        ExecuteMsg::SetNFTsWanted {
            trade_id,
            nfts_wanted,
        } => set_nfts_wanted(deps, env, info, trade_id, nfts_wanted),
        ExecuteMsg::FlushNFTsWanted { trade_id } => flush_nfts_wanted(deps, env, info, trade_id),
        ExecuteMsg::AddTokensWanted {
            trade_id,
            tokens_wanted,
        } => add_tokens_wanted(deps, env, info, trade_id, tokens_wanted),
        ExecuteMsg::RemoveTokensWanted {
            trade_id,
            tokens_wanted,
        } => remove_tokens_wanted(deps, env, info, trade_id, tokens_wanted),
        ExecuteMsg::SetTokensWanted {
            trade_id,
            tokens_wanted,
        } => set_tokens_wanted(deps, env, info, trade_id, tokens_wanted),
        ExecuteMsg::FlushTokensWanted { trade_id } => {
            flush_tokens_wanted(deps, env, info, trade_id)
        }
        ExecuteMsg::SetTradePreview { action, asset } => {
            set_trade_preview(deps, env, info, action, asset)
        }
        ExecuteMsg::SetComment {
            trade_id,
            counter_id,
            comment,
        } => set_comment(deps, env, info, trade_id, counter_id, comment),
        ExecuteMsg::ConfirmTrade { trade_id } => confirm_trade(deps, env, info, trade_id),
        ExecuteMsg::SuggestCounterTrade { trade_id, comment } => {
            suggest_counter_trade(deps, env, info, trade_id, comment)
        }
        ExecuteMsg::ConfirmCounterTrade {
            trade_id,
            counter_id,
        } => confirm_counter_trade(deps, env, info, trade_id, counter_id),
        ExecuteMsg::AcceptTrade {
            trade_id,
            counter_id,
            comment,
        } => accept_trade(deps, env, info, trade_id, counter_id, comment),
        ExecuteMsg::CancelTrade { trade_id } => cancel_trade(deps, env, info, trade_id),
        ExecuteMsg::CancelCounterTrade {
            trade_id,
            counter_id,
        } => cancel_counter_trade(deps, env, info, trade_id, counter_id),
        ExecuteMsg::RefuseCounterTrade {
            trade_id,
            counter_id,
        } => refuse_counter_trade(deps, env, info, trade_id, counter_id),
        ExecuteMsg::ReviewCounterTrade {
            trade_id,
            counter_id,
            comment,
        } => review_counter_trade(deps, env, info, trade_id, counter_id, comment),
        ExecuteMsg::WithdrawAllFromTrade { trade_id } => {
            withdraw_all_from_trade(deps, env, info, trade_id)
        }
        ExecuteMsg::WithdrawAllFromCounter {
            trade_id,
            counter_id,
        } => withdraw_all_from_counter(deps, env, info, trade_id, counter_id),
        ExecuteMsg::SetNewOwner { owner } => set_new_owner(deps, env, info, owner),
        ExecuteMsg::SetNewTreasury { treasury } => set_new_treasury(deps, env, info, treasury),
        ExecuteMsg::SetNewAcceptFee { accept_fee } => {
            set_new_accept_fee(deps, env, info, accept_fee)
        }
        ExecuteMsg::WithdrawSuccessfulTrade { trade_id } => {
            withdraw_successful_trade(deps, env, info, trade_id)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // No state migrations performed, just returned a Response
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractInfo {} => to_json_binary(&query_contract_info(deps)?),
        QueryMsg::TradeInfo { trade_id } => to_json_binary(
            &query_trade(deps.storage, trade_id)
                .map_err(|e| StdError::generic_err(e.to_string()))?,
        ),
        QueryMsg::CounterTradeInfo {
            trade_id,
            counter_id,
        } => to_json_binary(
            &query_counter_trade(deps.storage, trade_id, counter_id)
                .map_err(|e| StdError::generic_err(e.to_string()))?,
        ),
        QueryMsg::GetAllCounterTrades {
            start_after,
            limit,
            filters,
        } => to_json_binary(&query_all_counter_trades(
            deps,
            start_after,
            limit,
            filters,
        )?),
        QueryMsg::GetCounterTrades {
            trade_id,
            start_after,
            limit,
            filters,
        } => to_json_binary(&query_counter_trades(
            deps,
            trade_id,
            start_after,
            limit,
            filters,
        )?),
        QueryMsg::GetAllTrades {
            start_after,
            limit,
            filters,
        } => to_json_binary(&query_all_trades(deps, start_after, limit, filters)?),
    }
}

/// Replace the current contract owner with the provided owner address
/// * `owner` must be a valid Terra address
/// The owner has limited power on this contract :
/// 1. Change the contract owner
/// 2. Change the fee contract
pub fn set_new_owner(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    let mut contract_info = is_owner(deps.storage, info.sender)?;

    let new_owner = deps.api.addr_validate(&new_owner)?;
    contract_info.owner = new_owner.clone();
    CONTRACT_INFO.save(deps.storage, &contract_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("parameter", "owner")
        .add_attribute("value", new_owner))
}

/// Replace the current fee_contract with the provided fee_contract address
/// * `treasury` must be a valid Terra address
pub fn set_new_treasury(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    treasury: String,
) -> Result<Response, ContractError> {
    let mut contract_info = is_owner(deps.storage, info.sender)?;

    let treasury = deps.api.addr_validate(&treasury)?;
    contract_info.treasury = treasury.clone();
    CONTRACT_INFO.save(deps.storage, &contract_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("parameter", "treasury")
        .add_attribute("value", treasury))
}

/// Replace the current fee price for accepting trades
/// * `treasury` must be a valid Terra address
pub fn set_new_accept_fee(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    accept_fee: Vec<Coin>,
) -> Result<Response, ContractError> {
    let mut contract_info = is_owner(deps.storage, info.sender)?;

    if accept_fee.iter().any(|c| c.amount.is_zero()) {
        return Err(StdError::generic_err("Fee can't be zero").into());
    }

    contract_info.accept_trade_fee.clone_from(&accept_fee);
    CONTRACT_INFO.save(deps.storage, &contract_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("parameter", "accept_fee_possibilities")
        .add_attribute("value", format!("{:?}", accept_fee)))
}

/// General handler to add an asset to a trade or a counter trade
#[allow(clippy::too_many_arguments)]
pub fn add_asset(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: AddAssetAction,
    asset: AssetInfo,
) -> Result<Response, ContractError> {
    // We implement 4 different cases here.
    match action {
        AddAssetAction::ToLastTrade {} => add_asset_to_trade(deps, env, info, None, asset),
        AddAssetAction::ToLastCounterTrade { trade_id } => {
            add_asset_to_counter_trade(deps, env, info, trade_id, None, asset)
        }
        AddAssetAction::ToTrade { trade_id } => {
            add_asset_to_trade(deps, env, info, Some(trade_id), asset)
        }
        AddAssetAction::ToCounterTrade {
            trade_id,
            counter_id,
        } => add_asset_to_counter_trade(deps, env, info, trade_id, Some(counter_id), asset),
    }
}

/// Remove some assets from a trade when creating it.
pub fn withdraw_assets_while_creating(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: Option<u64>,
    assets: Vec<(u16, AssetInfo)>, // We chose to number the withdrawn assets to prevent looping over all deposited assets
) -> Result<Response, ContractError> {
    match counter_id {
        Some(counter_id) => withdraw_counter_trade_assets_while_creating(
            deps, env, info, trade_id, counter_id, assets,
        ),
        None => withdraw_trade_assets_while_creating(deps, env, info, trade_id, assets),
    }
}

/// Withdraw assets from an accepted trade.
/// The trader will withdraw assets from the counter_trade
/// The counter_trader will withdraw assets from the trade
pub fn withdraw_successful_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
) -> Result<Response, ContractError> {
    // We load the trade and verify it has been accepted
    let mut trade_info = load_trade(deps.storage, trade_id)?;
    if trade_info.state != TradeState::Accepted {
        return Err(ContractError::TradeNotAccepted {});
    }

    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    // We check that they pay the right fee
    let funds = assert_payment(&info, contract_info.accept_trade_fee)?;

    // We load the corresponding counter_trade
    let counter_id = trade_info
        .accepted_info
        .clone()
        .ok_or(ContractError::ContractBug {})?
        .counter_id;
    let mut counter_info = load_counter_trade(deps.storage, trade_id, counter_id)?;

    let (res, trade_type);

    // We indentify who the transaction sender is (trader or counter-trader)
    if trade_info.owner == info.sender {
        // In case the trader wants to withdraw the exchanged funds (from the counter_info object)
        let royalties = gather_royalties(deps.as_ref(), &trade_info)?;
        res = check_and_create_withdraw_messages(
            deps.as_ref(),
            &info.sender,
            &counter_info,
            Some(royalties),
            Some(contract_info.fund_fee),
        )?;

        trade_type = "counter";
        counter_info.assets_withdrawn = true;
        COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &counter_info)?;
    } else if counter_info.owner == info.sender {
        // In case the counter_trader wants to withdraw the exchanged funds (from the trade_info object)
        let royalties = gather_royalties(deps.as_ref(), &counter_info)?;
        res = check_and_create_withdraw_messages(
            deps.as_ref(),
            &info.sender,
            &trade_info,
            Some(royalties),
            Some(contract_info.fund_fee),
        )?;

        trade_type = "trade";
        trade_info.assets_withdrawn = true;
        TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;
    } else {
        return Err(ContractError::NotWithdrawableByYou {});
    }

    Ok(res
        .add_message(BankMsg::Send {
            to_address: contract_info.treasury.to_string(),
            amount: vec![funds],
        })
        .add_attribute("action", "withdraw_funds")
        .add_attribute("type", trade_type)
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", counter_info.owner))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::state::load_trade;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Addr, Attribute, BankMsg, Coin, Decimal, Uint128};
    use cw1155::Cw1155ExecuteMsg;
    use cw20::Cw20ExecuteMsg;
    use cw721::Cw721ExecuteMsg;
    use p2p_trading_export::msg::into_cosmos_msg;

    fn init_helper(deps: DepsMut) {
        let instantiate_msg = InstantiateMsg {
            name: "p2p-trading".to_string(),
            owner: None,
            accept_trade_fee: coins(2367, "ujuno"),
            treasury: "treasury".to_string(),
            fund_fee: Decimal::percent(3),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();

        instantiate(deps, env, info, instantiate_msg).unwrap();
    }

    fn set_treasury_helper(deps: DepsMut) {
        let info = mock_info("creator", &[]);
        let env = mock_env();
        execute(
            deps,
            env,
            info,
            ExecuteMsg::SetNewTreasury {
                treasury: "treasury".to_string(),
            },
        )
        .unwrap();
    }

    #[test]
    fn test_init_sanity() {
        let mut deps = mock_dependencies();
        let instantiate_msg = InstantiateMsg {
            name: "p2p-trading".to_string(),
            owner: Some("this_address".to_string()),
            accept_trade_fee: coins(2367, "ujuno"),
            treasury: "treasury".to_string(),
            fund_fee: Decimal::percent(3),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();

        let res_init = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
        assert_eq!(0, res_init.messages.len());
    }

    #[test]
    fn test_change_owner() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &[]);
        let env = mock_env();
        init_helper(deps.as_mut());

        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::SetNewOwner {
                owner: "new_owner".to_string(),
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            info,
            ExecuteMsg::SetNewOwner {
                owner: "new_owner".to_string(),
            },
        )
        .unwrap_err();
        let info = mock_info("new_owner", &[]);
        execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::SetNewOwner {
                owner: "other_owner".to_string(),
            },
        )
        .unwrap();
    }

    fn create_trade_helper(deps: DepsMut, creator: &str) -> Response {
        let info = mock_info(creator, &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::CreateTrade {
                whitelisted_users: Some(vec![]),
                comment: Some("Q".to_string()),
            },
        )
        .unwrap()
    }

    fn create_private_trade_helper(deps: DepsMut, users: Vec<String>) -> Response {
        let info = mock_info("creator", &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::CreateTrade {
                whitelisted_users: Some(users),
                comment: None,
            },
        )
        .unwrap()
    }

    fn add_whitelisted_users(
        deps: DepsMut,
        trade_id: u64,
        users: Vec<String>,
    ) -> Result<Response, ContractError> {
        let info = mock_info("creator", &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::AddWhitelistedUsers {
                trade_id,
                whitelisted_users: users,
            },
        )
    }

    fn remove_whitelisted_users(
        deps: DepsMut,
        trade_id: u64,
        users: Vec<String>,
    ) -> Result<Response, ContractError> {
        let info = mock_info("creator", &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::RemoveWhitelistedUsers {
                trade_id,
                whitelisted_users: users,
            },
        )
    }

    fn add_nfts_wanted_helper(
        deps: DepsMut,
        trader: &str,
        trade_id: u64,
        confirm: Vec<String>,
    ) -> Result<Response, ContractError> {
        let info = mock_info(trader, &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::AddNFTsWanted {
                trade_id: Some(trade_id),
                nfts_wanted: confirm,
            },
        )
    }

    fn remove_nfts_wanted_helper(
        deps: DepsMut,
        trader: &str,
        trade_id: u64,
        confirm: Vec<String>,
    ) -> Result<Response, ContractError> {
        let info = mock_info(trader, &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::RemoveNFTsWanted {
                trade_id,
                nfts_wanted: confirm,
            },
        )
    }

    fn add_asset_to_trade_helper(
        deps: DepsMut,
        trader: &str,
        trade_id: u64,
        asset: AssetInfo,
        coins_to_send: &[Coin],
    ) -> Result<Response, ContractError> {
        let info = mock_info(trader, coins_to_send);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::AddAsset {
                action: AddAssetAction::ToTrade { trade_id },
                asset,
            },
        )
    }

    fn remove_assets_helper(
        deps: DepsMut,
        sender: &str,
        trade_id: u64,
        counter_id: Option<u64>,
        assets: Vec<(u16, AssetInfo)>,
    ) -> Result<Response, ContractError> {
        let info = mock_info(sender, &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::RemoveAssets {
                trade_id,
                counter_id,
                assets,
            },
        )
    }

    fn confirm_trade_helper(
        deps: DepsMut,
        sender: &str,
        trade_id: u64,
    ) -> Result<Response, ContractError> {
        let info = mock_info(sender, &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::ConfirmTrade {
                trade_id: Some(trade_id),
            },
        )
    }

    fn withdraw_cancelled_trade_helper(
        deps: DepsMut,
        sender: &str,
        trade_id: u64,
    ) -> Result<Response, ContractError> {
        let info = mock_info(sender, &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::WithdrawAllFromTrade { trade_id },
        )
    }

    fn withdraw_aborted_counter_helper(
        deps: DepsMut,
        sender: &str,
        trade_id: u64,
        counter_id: u64,
    ) -> Result<Response, ContractError> {
        let info = mock_info(sender, &[]);
        let env = mock_env();

        execute(
            deps,
            env,
            info,
            ExecuteMsg::WithdrawAllFromCounter {
                trade_id,
                counter_id,
            },
        )
    }

    // pub mod trade_tests {
    //     use super::*;
    //     use crate::query::{query_counter_trades, TradeResponse};
    //     use crate::trade::validate_addresses;
    //     use cosmwasm_std::{coin, Api, SubMsg};
    //     use p2p_trading_export::msg::{
    //         AdditionalTradeInfoResponse, QueryFilters, TradeInfoResponse,
    //     };
    //     use p2p_trading_export::state::{Comment, CounterTradeInfo};
    //     use std::collections::HashSet;
    //     use std::iter::FromIterator;
    //     use utils::state::Cw721Coin;

    //     #[test]
    //     fn create_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         let res = create_trade_helper(deps.as_mut(), "creator");

    //         assert_eq!(res.messages, vec![]);
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "create_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //             ]
    //         );

    //         let res = create_trade_helper(deps.as_mut(), "creator");

    //         assert_eq!(res.messages, vec![]);
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "create_trade"),
    //                 Attribute::new("trade_id", "1"),
    //                 Attribute::new("trader", "creator"),
    //             ]
    //         );

    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();

    //         assert_eq!(new_trade_info.state, TradeState::Created {});

    //         // Query all and check that trades exist, without filters specified
    //         let res = query_all_trades(deps.as_ref(), None, None, None).unwrap();

    //         assert_eq!(
    //             res.trades,
    //             vec![
    //                 {
    //                     TradeResponse {
    //                         trade_id: 1,
    //                         counter_id: None,
    //                         trade_info: Some(TradeInfoResponse {
    //                             owner: deps.api.addr_validate("creator").unwrap(),
    //                             additional_info: AdditionalTradeInfoResponse {
    //                                 owner_comment: Some(Comment {
    //                                     comment: "Q".to_string(),
    //                                     time: mock_env().block.time,
    //                                 }),
    //                                 time: mock_env().block.time,
    //                                 ..Default::default()
    //                             },
    //                             ..Default::default()
    //                         }),
    //                     }
    //                 },
    //                 {
    //                     TradeResponse {
    //                         trade_id: 0,
    //                         counter_id: None,
    //                         trade_info: Some(TradeInfoResponse {
    //                             owner: deps.api.addr_validate("creator").unwrap(),
    //                             additional_info: AdditionalTradeInfoResponse {
    //                                 owner_comment: Some(Comment {
    //                                     comment: "Q".to_string(),
    //                                     time: mock_env().block.time,
    //                                 }),
    //                                 time: mock_env().block.time,
    //                                 ..Default::default()
    //                             },
    //                             ..Default::default()
    //                         }),
    //                     }
    //                 }
    //             ]
    //         );
    //     }

    //     #[test]
    //     fn create_trade_and_nfts_wanted() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         let res = add_nfts_wanted_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             vec!["nft1".to_string(), "nft2".to_string()],
    //         )
    //         .unwrap();
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "modify_parameter"),
    //                 Attribute::new("name", "nfts_wanted"),
    //                 Attribute::new("operation_type", "add"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("trader", "creator")
    //             ]
    //         );

    //         let trade = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(
    //             trade.additional_info.nfts_wanted,
    //             HashSet::from_iter(vec![Addr::unchecked("nft1"), Addr::unchecked("nft2")])
    //         );

    //         add_nfts_wanted_helper(deps.as_mut(), "creator", 0, vec!["nft1".to_string()]).unwrap();
    //         remove_nfts_wanted_helper(deps.as_mut(), "creator", 0, vec!["nft1".to_string()])
    //             .unwrap();

    //         let trade = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(
    //             trade.additional_info.nfts_wanted,
    //             HashSet::from_iter(vec![Addr::unchecked("nft2")])
    //         );
    //     }

    //     #[test]
    //     fn create_multiple_trades_and_query() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         let res = create_trade_helper(deps.as_mut(), "creator");

    //         assert_eq!(res.messages, vec![]);
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "create_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //             ]
    //         );

    //         let res = create_trade_helper(deps.as_mut(), "creator2");

    //         assert_eq!(res.messages, vec![]);
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "create_trade"),
    //                 Attribute::new("trade_id", "1"),
    //                 Attribute::new("trader", "creator2"),
    //             ]
    //         );

    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();

    //         assert_eq!(new_trade_info.state, TradeState::Created {});

    //         let new_trade_info = load_trade(&deps.storage, 1).unwrap();
    //         assert_eq!(new_trade_info.state, TradeState::Created {});

    //         create_trade_helper(deps.as_mut(), "creator2");
    //         confirm_trade_helper(deps.as_mut(), "creator2", 2).unwrap();

    //         // Query all created trades check that creators are different
    //         let res = query_all_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![TradeState::Created.to_string()]),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.trades,
    //             vec![
    //                 {
    //                     TradeResponse {
    //                         trade_id: 1,
    //                         counter_id: None,
    //                         trade_info: Some(TradeInfoResponse {
    //                             owner: deps.api.addr_validate("creator2").unwrap(),
    //                             additional_info: AdditionalTradeInfoResponse {
    //                                 owner_comment: Some(Comment {
    //                                     comment: "Q".to_string(),
    //                                     time: mock_env().block.time,
    //                                 }),
    //                                 time: mock_env().block.time,
    //                                 ..Default::default()
    //                             },
    //                             ..Default::default()
    //                         }),
    //                     }
    //                 },
    //                 {
    //                     TradeResponse {
    //                         trade_id: 0,
    //                         counter_id: None,
    //                         trade_info: Some(TradeInfoResponse {
    //                             owner: deps.api.addr_validate("creator").unwrap(),
    //                             additional_info: AdditionalTradeInfoResponse {
    //                                 owner_comment: Some(Comment {
    //                                     comment: "Q".to_string(),
    //                                     time: mock_env().block.time,
    //                                 }),
    //                                 time: mock_env().block.time,
    //                                 ..Default::default()
    //                             },
    //                             ..Default::default()
    //                         }),
    //                     }
    //                 }
    //             ]
    //         );

    //         // Verify that pagination by trade_id works
    //         let res = query_all_trades(
    //             deps.as_ref(),
    //             Some(1),
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![TradeState::Created.to_string()]),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.trades,
    //             vec![{
    //                 TradeResponse {
    //                     trade_id: 0,
    //                     counter_id: None,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("creator").unwrap(),
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     }),
    //                 }
    //             }]
    //         );

    //         // Query that query returned only queries that are in created state and belong to creator2
    //         let res = query_all_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![TradeState::Created.to_string()]),
    //                 owner: Some("creator2".to_string()),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.trades,
    //             vec![TradeResponse {
    //                 trade_id: 1,
    //                 counter_id: None,
    //                 trade_info: Some(TradeInfoResponse {
    //                     owner: deps.api.addr_validate("creator2").unwrap(),
    //                     additional_info: AdditionalTradeInfoResponse {
    //                         owner_comment: Some(Comment {
    //                             comment: "Q".to_string(),
    //                             time: mock_env().block.time
    //                         }),
    //                         time: mock_env().block.time,
    //                         ..Default::default()
    //                     },
    //                     ..Default::default()
    //                 })
    //             }]
    //         );

    //         // Check that if states are None that owner query still works
    //         let res = query_all_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 owner: Some("creator2".to_string()),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.trades,
    //             vec![
    //                 TradeResponse {
    //                     trade_id: 2,
    //                     counter_id: None,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("creator2").unwrap(),
    //                         state: TradeState::Published,
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     })
    //                 },
    //                 TradeResponse {
    //                     trade_id: 1,
    //                     counter_id: None,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("creator2").unwrap(),
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     })
    //                 }
    //             ]
    //         );

    //         // Check that queries with published state do not return anything. Because none exists.
    //         let res = query_all_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![TradeState::Accepted.to_string()]),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(res.trades, vec![]);

    //         // Check that queries with published state do not return anything when owner is specified. Because none exists.
    //         let res = query_all_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![TradeState::Accepted.to_string()]),
    //                 owner: Some("creator2".to_string()),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();
    //         assert_eq!(res.trades, vec![]);
    //     }

    //     #[test]
    //     fn create_trade_and_add_funds() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         let res = add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Coin(coin(2, "token")),
    //             &coins(2, "token"),
    //         )
    //         .unwrap();
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "add_asset"),
    //                 Attribute::new("asset_type", "fund"),
    //                 Attribute::new("denom", "token"),
    //                 Attribute::new("amount", "2"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //             ]
    //         );

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Coin(coin(2, "token")),
    //             &coins(2, "token"),
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Coin(coin(2, "other_token")),
    //             &coins(2, "other_token"),
    //         )
    //         .unwrap();

    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(
    //             new_trade_info.associated_assets,
    //             vec![
    //                 AssetInfo::Coin(Coin {
    //                     amount: Uint128::from(4u64),
    //                     denom: "token".to_string()
    //                 }),
    //                 AssetInfo::Coin(Coin {
    //                     amount: Uint128::from(2u64),
    //                     denom: "other_token".to_string()
    //                 })
    //             ]
    //         );
    //     }

    //     #[test]
    //     fn create_trade_and_add_cw721_tokens() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");

    //         let res = add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "add_asset"),
    //                 Attribute::new("asset_type", "NFT"),
    //                 Attribute::new("nft", "nft"),
    //                 Attribute::new("token_id", "58"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //             ]
    //         );

    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(
    //             new_trade_info.associated_assets,
    //             vec![AssetInfo::Cw721Coin(Cw721Coin {
    //                 token_id: "58".to_string(),
    //                 address: "nft".to_string()
    //             })]
    //         );

    //         // This triggers an error, the creator is not the same as the sender
    //         let err = add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "bad_person",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "other_token".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::TraderNotCreator {});
    //     }

    //     #[test]
    //     fn create_trade_and_withdraw() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         create_trade_helper(deps.as_mut(), "creator");
    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "1155".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();
    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "other_token".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         withdraw_cancelled_trade_helper(deps.as_mut(), "bas_person", 0).unwrap_err();
    //         withdraw_cancelled_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(new_trade_info.state, TradeState::Cancelled);

    //         withdraw_cancelled_trade_helper(deps.as_mut(), "creator", 0).unwrap_err();
    //     }
    //     #[test]
    //     fn create_trade_automatic_trade_id() {
    //         let mut deps = mock_dependencies();
    //         let info = mock_info("creator", &[]);
    //         let env = mock_env();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         create_trade_helper(deps.as_mut(), "creator");

    //         execute(
    //             deps.as_mut(),
    //             env.clone(),
    //             info,
    //             ExecuteMsg::AddAsset {
    //                 action: AddAssetAction::ToLastTrade {},
    //                 asset: AssetInfo::Coin(Coin {
    //                     denom: "cw20".to_string(),
    //                     amount: Uint128::from(100u64),
    //                 }),
    //             },
    //         )
    //         .unwrap();

    //         let info = mock_info("creator", &coins(97u128, "uluna"));
    //         execute(
    //             deps.as_mut(),
    //             env,
    //             info,
    //             ExecuteMsg::AddAsset {
    //                 action: AddAssetAction::ToLastTrade {},
    //                 asset: AssetInfo::Coin(coin(97u128, "uluna")),
    //             },
    //         )
    //         .unwrap();

    //         let trade_info = TRADE_INFO.load(&deps.storage, 1u64).unwrap();
    //         assert_eq!(
    //             trade_info.associated_assets,
    //             vec![AssetInfo::Coin(coin(97u128, "uluna")),]
    //         );
    //     }

    //     #[test]
    //     fn create_trade_add_remove_tokens() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft-2".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Coin(coin(100, "luna")),
    //             &coins(100, "luna"),
    //         )
    //         .unwrap();

    //         let res = remove_assets_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             None,
    //             vec![
    //                 (
    //                     0,
    //                     AssetInfo::Cw721Coin(Cw721Coin {
    //                         address: "nft".to_string(),
    //                         token_id: "58".to_string(),
    //                     }),
    //                 ),
    //                 (4, AssetInfo::Coin(coin(58, "luna"))),
    //             ],
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.messages,
    //             vec![
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw721ExecuteMsg::TransferNft {
    //                             recipient: "creator".to_string(),
    //                             token_id: "58".to_string()
    //                         },
    //                         "nft"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw1155ExecuteMsg::SendFrom {
    //                             from: mock_env().contract.address.to_string(),
    //                             to: "creator".to_string(),
    //                             token_id: "58".to_string(),
    //                             value: Uint128::from(58u128),
    //                             msg: None
    //                         },
    //                         "cw1155token"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw20ExecuteMsg::Transfer {
    //                             recipient: "creator".to_string(),
    //                             amount: Uint128::from(58u64)
    //                         },
    //                         "token"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(BankMsg::Send {
    //                     to_address: "creator".to_string(),
    //                     amount: coins(58, "luna"),
    //                 })
    //             ]
    //         );

    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(
    //             new_trade_info.associated_assets,
    //             vec![
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     token_id: "58".to_string(),
    //                     address: "nft-2".to_string()
    //                 }),
    //                 AssetInfo::Cw1155Coin(Cw1155Coin {
    //                     value: Uint128::from(42u64),
    //                     address: "cw1155token".to_string(),
    //                     token_id: "58".to_string()
    //                 }),
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     amount: Uint128::from(42u64),
    //                     address: "token".to_string()
    //                 }),
    //                 AssetInfo::Coin(coin(42, "luna"))
    //             ],
    //         );

    //         remove_assets_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             None,
    //             vec![
    //                 (
    //                     1,
    //                     AssetInfo::Cw1155Coin(Cw1155Coin {
    //                         address: "cw1155token".to_string(),
    //                         token_id: "58".to_string(),
    //                         value: Uint128::from(42u64),
    //                     }),
    //                 ),
    //                 (
    //                     2,
    //                     AssetInfo::Cw20Coin(Cw20Coin {
    //                         address: "token".to_string(),
    //                         amount: Uint128::from(42u64),
    //                     }),
    //                 ),
    //                 (3, AssetInfo::Coin(coin(42, "luna"))),
    //             ],
    //         )
    //         .unwrap();

    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(
    //             new_trade_info.associated_assets,
    //             vec![AssetInfo::Cw721Coin(Cw721Coin {
    //                 token_id: "58".to_string(),
    //                 address: "nft-2".to_string()
    //             }),],
    //         );

    //         // This triggers an error, the creator is not the same as the sender
    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "bad_person",
    //             0,
    //             None,
    //             vec![(
    //                 0,
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     address: "nft-2".to_string(),
    //                     token_id: "58".to_string(),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::TraderNotCreator {});

    //         // This triggers an error, no matching funds were found
    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             None,
    //             vec![(
    //                 1,
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     address: "nft-2".to_string(),
    //                     token_id: "58".to_string(),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 1 });

    //         // This triggers an error, no matching funds were found
    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             None,
    //             vec![(
    //                 0,
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     address: "nft-1".to_string(),
    //                     token_id: "58".to_string(),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 0 });

    //         // This triggers an error, no matching funds were found
    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             None,
    //             vec![(
    //                 0,
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     address: "nft-2".to_string(),
    //                     token_id: "42".to_string(),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 0 });
    //     }

    //     #[test]
    //     fn create_trade_add_remove_tokens_errors() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft-2".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Coin(coin(100, "luna")),
    //             &coins(100, "luna"),
    //         )
    //         .unwrap();

    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             None,
    //             vec![(
    //                 2,
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "token".to_string(),
    //                     amount: Uint128::from(101u64),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(
    //             err,
    //             ContractError::TooMuchWithdrawn {
    //                 address: "token".to_string(),
    //                 wanted: 101,
    //                 available: 100
    //             }
    //         );

    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             None,
    //             vec![(
    //                 0,
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "token".to_string(),
    //                     amount: Uint128::from(101u64),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 0 });

    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             None,
    //             vec![(
    //                 2,
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "wrong-token".to_string(),
    //                     amount: Uint128::from(101u64),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 2 });

    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             None,
    //             vec![(
    //                 2,
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "token".to_string(),
    //                     amount: Uint128::from(58u64),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::TradeAlreadyPublished {});
    //     }

    //     #[test]
    //     fn confirm_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");

    //         //Wrong trade id
    //         let err = confirm_trade_helper(deps.as_mut(), "creator", 1).unwrap_err();
    //         assert_eq!(err, ContractError::NotFoundInTradeInfo {});

    //         //Wrong trader
    //         let err = confirm_trade_helper(deps.as_mut(), "bad_person", 0).unwrap_err();
    //         assert_eq!(err, ContractError::TraderNotCreator {});

    //         let res = confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "confirm_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //             ]
    //         );

    //         // Check with query that trade is confirmed, in published state
    //         let res = query_all_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![TradeState::Published.to_string()]),
    //                 owner: Some("creator".to_string()),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.trades,
    //             vec![{
    //                 TradeResponse {
    //                     trade_id: 0,
    //                     counter_id: None,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("creator").unwrap(),
    //                         state: TradeState::Published,
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     }),
    //                 }
    //             }]
    //         );

    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(new_trade_info.state, TradeState::Published {});

    //         //Already confirmed
    //         let err = confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap_err();
    //         assert_eq!(
    //             err,
    //             ContractError::CantChangeTradeState {
    //                 from: TradeState::Published,
    //                 to: TradeState::Published
    //             }
    //         );
    //     }

    //     #[test]
    //     fn confirm_trade_and_try_add_assets() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");

    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         // This triggers an error, we can't send funds to confirmed trade

    //         let err = add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Coin(coin(2, "token")),
    //             &coins(2, "token"),
    //         )
    //         .unwrap_err();

    //         assert_eq!(
    //             err,
    //             ContractError::WrongTradeState {
    //                 state: TradeState::Published
    //             }
    //         );
    //     }

    //     #[test]
    //     fn accept_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         let err = accept_trade_helper(deps.as_mut(), "creator", 0, 5).unwrap_err();
    //         assert_eq!(err, ContractError::NotFoundInCounterTradeInfo {});

    //         let err = accept_trade_helper(deps.as_mut(), "creator", 1, 0).unwrap_err();
    //         assert_eq!(err, ContractError::NotFoundInTradeInfo {});

    //         let err = accept_trade_helper(deps.as_mut(), "bad_person", 0, 0).unwrap_err();
    //         assert_eq!(err, ContractError::TraderNotCreator {});

    //         let err = accept_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap_err();
    //         assert_eq!(err, ContractError::CantAcceptNotPublishedCounter {});

    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();

    //         let res = accept_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();

    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "accept_counter_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );

    //         let trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(trade_info.state, TradeState::Accepted {});
    //         assert_eq!(
    //             trade_info.accepted_info.unwrap(),
    //             CounterTradeInfo {
    //                 trade_id: 0,
    //                 counter_id: 0
    //             }
    //         );

    //         let counter_trade_info = load_counter_trade(&deps.storage, 0, 0).unwrap();
    //         assert_eq!(counter_trade_info.state, TradeState::Accepted {});

    //         // Check with query that trade is confirmed, in ack state
    //         let res = query_all_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![TradeState::Accepted.to_string()]),
    //                 owner: Some("creator".to_string()),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.trades,
    //             vec![{
    //                 TradeResponse {
    //                     trade_id: 0,
    //                     counter_id: None,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("creator").unwrap(),
    //                         state: TradeState::Accepted,
    //                         last_counter_id: Some(0),
    //                         accepted_info: Some(CounterTradeInfo {
    //                             trade_id: 0,
    //                             counter_id: 0,
    //                         }),
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     }),
    //                 }
    //             }]
    //         );

    //         // Check with query by trade id that one counter is returned
    //         let res = query_counter_trades(deps.as_ref(), 0, None, None, None).unwrap();

    //         assert_eq!(
    //             res.counter_trades,
    //             vec![{
    //                 TradeResponse {
    //                     counter_id: Some(0),
    //                     trade_id: 0,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("counterer").unwrap(),
    //                         state: TradeState::Accepted,
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             trader_comment: Some(Comment {
    //                                 comment: "You're very kind madam".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     }),
    //                 }
    //             }]
    //         );

    //         let res = query_counter_trades(deps.as_ref(), 0, Some(0), None, None).unwrap();
    //         assert_eq!(res.counter_trades, vec![]);

    //         // Check with queries that only one counter is returned by query and in accepted state
    //         let res = query_all_counter_trades(deps.as_ref(), None, None, None).unwrap();

    //         assert_eq!(
    //             res.counter_trades,
    //             vec![{
    //                 TradeResponse {
    //                     counter_id: Some(0),
    //                     trade_id: 0,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("counterer").unwrap(),
    //                         state: TradeState::Accepted,
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             trader_comment: Some(Comment {
    //                                 comment: "You're very kind madam".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     }),
    //                 }
    //             }]
    //         );
    //     }

    //     #[test]
    //     fn accept_trade_with_multiple_counter() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 1).unwrap();

    //         let res = accept_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();

    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "accept_counter_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );

    //         let trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(trade_info.state, TradeState::Accepted {});

    //         let counter_trade_info = load_counter_trade(&deps.storage, 0, 0).unwrap();
    //         assert_eq!(counter_trade_info.state, TradeState::Accepted {});

    //         let counter_trade_info = load_counter_trade(&deps.storage, 0, 1).unwrap();
    //         assert_eq!(counter_trade_info.state, TradeState::Refused {});

    //         // Check that the only Accepted and Published counters are the accepted counter
    //         let res = query_all_counter_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![
    //                     TradeState::Accepted.to_string(),
    //                     TradeState::Published.to_string(),
    //                 ]),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.counter_trades,
    //             vec![{
    //                 TradeResponse {
    //                     counter_id: Some(0),
    //                     trade_id: 0,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("counterer").unwrap(),
    //                         state: TradeState::Accepted,

    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             trader_comment: Some(Comment {
    //                                 comment: "You're very kind madam".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     }),
    //                 }
    //             }]
    //         );
    //         // Check that the other counters is cancelled
    //         let res = query_all_counter_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![TradeState::Refused.to_string()]),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.counter_trades,
    //             vec![
    //                 {
    //                     TradeResponse {
    //                         counter_id: Some(2),
    //                         trade_id: 0,
    //                         trade_info: Some(TradeInfoResponse {
    //                             owner: deps.api.addr_validate("counterer").unwrap(),
    //                             state: TradeState::Refused,
    //                             additional_info: AdditionalTradeInfoResponse {
    //                                 owner_comment: Some(Comment {
    //                                     comment: "Q".to_string(),
    //                                     time: mock_env().block.time,
    //                                 }),
    //                                 time: mock_env().block.time,
    //                                 ..Default::default()
    //                             },
    //                             ..Default::default()
    //                         }),
    //                     }
    //                 },
    //                 {
    //                     TradeResponse {
    //                         counter_id: Some(1),
    //                         trade_id: 0,
    //                         trade_info: Some(TradeInfoResponse {
    //                             owner: deps.api.addr_validate("counterer").unwrap(),
    //                             state: TradeState::Refused,
    //                             additional_info: AdditionalTradeInfoResponse {
    //                                 owner_comment: Some(Comment {
    //                                     comment: "Q".to_string(),
    //                                     time: mock_env().block.time,
    //                                 }),
    //                                 time: mock_env().block.time,
    //                                 ..Default::default()
    //                             },
    //                             ..Default::default()
    //                         }),
    //                     }
    //                 },
    //             ]
    //         );

    //         // Check that both Accepted and Published counter queries exist, paginate to skip last counter trade
    //         let res = query_all_counter_trades(
    //             deps.as_ref(),
    //             Some(CounterTradeInfo {
    //                 trade_id: 0,
    //                 counter_id: 1,
    //             }),
    //             None,
    //             Some(QueryFilters {
    //                 states: Some(vec![
    //                     TradeState::Accepted.to_string(),
    //                     TradeState::Published.to_string(),
    //                 ]),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.counter_trades,
    //             vec![{
    //                 TradeResponse {
    //                     counter_id: Some(0),
    //                     trade_id: 0,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("counterer").unwrap(),
    //                         state: TradeState::Accepted,
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             trader_comment: Some(Comment {
    //                                 comment: "You're very kind madam".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     }),
    //                 }
    //             }]
    //         );
    //     }

    //     #[test]
    //     fn cancel_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         let err = cancel_trade_helper(deps.as_mut(), "creator", 1).unwrap_err();
    //         assert_eq!(err, ContractError::NotFoundInTradeInfo {});

    //         let err = cancel_trade_helper(deps.as_mut(), "bad_person", 0).unwrap_err();
    //         assert_eq!(err, ContractError::TraderNotCreator {});

    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();

    //         let res = cancel_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "cancel_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //             ]
    //         );

    //         // Query all counter trades make sure counter trade is cancelled with the trade
    //         let res = query_all_counter_trades(deps.as_ref(), None, None, None).unwrap();

    //         assert_eq!(
    //             res.counter_trades,
    //             vec![{
    //                 TradeResponse {
    //                     counter_id: Some(0),
    //                     trade_id: 0,
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("counterer").unwrap(),
    //                         state: TradeState::Cancelled,
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time,
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     }),
    //                 }
    //             }]
    //         );
    //     }

    //     #[test]
    //     fn queries_with_multiple_trades_and_counter_trades() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 1).unwrap();

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 2).unwrap();

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 3).unwrap();

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 4).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer2", 0).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 1).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 2).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 3).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 4).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer2", 4).unwrap();

    //         // Query all before second one, should return the first one
    //         let res = query_all_counter_trades(
    //             deps.as_ref(),
    //             Some(CounterTradeInfo {
    //                 trade_id: 0,
    //                 counter_id: 1,
    //             }),
    //             None,
    //             Some(QueryFilters {
    //                 owner: Some("counterer2".to_string()),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.counter_trades,
    //             vec![TradeResponse {
    //                 trade_id: 0,
    //                 counter_id: Some(0),
    //                 trade_info: Some(TradeInfoResponse {
    //                     owner: deps.api.addr_validate("counterer2").unwrap(),
    //                     state: TradeState::Created,
    //                     additional_info: AdditionalTradeInfoResponse {
    //                         owner_comment: Some(Comment {
    //                             comment: "Q".to_string(),
    //                             time: mock_env().block.time
    //                         }),
    //                         time: mock_env().block.time,
    //                         ..Default::default()
    //                     },
    //                     ..Default::default()
    //                 })
    //             }]
    //         );

    //         // Query all before first one, should return empty array
    //         let res = query_all_counter_trades(
    //             deps.as_ref(),
    //             Some(CounterTradeInfo {
    //                 trade_id: 0,
    //                 counter_id: 0,
    //             }),
    //             None,
    //             None,
    //         )
    //         .unwrap();

    //         assert_eq!(res.counter_trades, vec![]);

    //         // Query for non existing user should return empty []
    //         let res = query_all_counter_trades(
    //             deps.as_ref(),
    //             None,
    //             None,
    //             Some(QueryFilters {
    //                 owner: Some("counterer5".to_string()),
    //                 ..Default::default()
    //             }),
    //         )
    //         .unwrap();

    //         assert_eq!(res.counter_trades, vec![]);

    //         // Query by trade_id should return counter queries for trade id 4
    //         let res = query_counter_trades(deps.as_ref(), 4, None, None, None).unwrap();

    //         assert_eq!(
    //             res.counter_trades,
    //             vec![
    //                 TradeResponse {
    //                     trade_id: 4,
    //                     counter_id: Some(1),
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("counterer2").unwrap(),
    //                         state: TradeState::Created,
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     })
    //                 },
    //                 TradeResponse {
    //                     trade_id: 4,
    //                     counter_id: Some(0),
    //                     trade_info: Some(TradeInfoResponse {
    //                         owner: deps.api.addr_validate("counterer").unwrap(),
    //                         additional_info: AdditionalTradeInfoResponse {
    //                             owner_comment: Some(Comment {
    //                                 comment: "Q".to_string(),
    //                                 time: mock_env().block.time
    //                             }),
    //                             time: mock_env().block.time,
    //                             ..Default::default()
    //                         },
    //                         ..Default::default()
    //                     })
    //                 }
    //             ]
    //         );
    //     }

    //     #[test]
    //     fn withdraw_accepted_assets() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());
    //         set_fee_contract_helper(deps.as_mut());
    //         create_trade_helper(deps.as_mut(), "creator");

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw1155Coin(Cw1155Coin {
    //                 address: "cw1155".to_string(),
    //                 token_id: "58".to_string(),
    //                 value: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Coin(coin(9, "other_token")),
    //             &coins(9, "other_token"),
    //         )
    //         .unwrap();

    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "other_counterer", 0).unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "other_counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "other_counter-nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "other_counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "other_counter-token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "other_counterer",
    //             0,
    //             0,
    //             AssetInfo::Coin(coin(5, "lunas")),
    //             &coins(5, "lunas"),
    //         )
    //         .unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             1,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "counter-nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             1,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "counter-token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             1,
    //             AssetInfo::Coin(coin(2, "token")),
    //             &coins(2, "token"),
    //         )
    //         .unwrap();

    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 1).unwrap();

    //         // Little test to start with (can't withdraw if the trade is not accepted)
    //         let err = withdraw_helper(deps.as_mut(), "anyone", "fee_contract", 0).unwrap_err();
    //         assert_eq!(err, ContractError::TradeNotAccepted {});

    //         accept_trade_helper(deps.as_mut(), "creator", 0, 1).unwrap();

    //         // Withdraw tests
    //         let err = withdraw_helper(deps.as_mut(), "bad_person", "fee_contract", 0).unwrap_err();
    //         assert_eq!(err, ContractError::NotWithdrawableByYou {});

    //         let err = withdraw_helper(deps.as_mut(), "creator", "bad_person", 0).unwrap_err();
    //         assert_eq!(err, ContractError::Unauthorized {});

    //         let res = withdraw_helper(deps.as_mut(), "creator", "fee_contract", 0).unwrap();
    //         assert_eq!(
    //             res.messages,
    //             vec![
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw721ExecuteMsg::TransferNft {
    //                             recipient: "creator".to_string(),
    //                             token_id: "58".to_string()
    //                         },
    //                         "counter-nft"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw20ExecuteMsg::Transfer {
    //                             recipient: "creator".to_string(),
    //                             amount: Uint128::from(100u64)
    //                         },
    //                         "counter-token"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(BankMsg::Send {
    //                     to_address: "creator".to_string(),
    //                     amount: coins(2, "token"),
    //                 })
    //             ]
    //         );

    //         let err = withdraw_helper(deps.as_mut(), "creator", "fee_contract", 0).unwrap_err();
    //         assert_eq!(err, ContractError::TradeAlreadyWithdrawn {});

    //         let res = withdraw_helper(deps.as_mut(), "counterer", "fee_contract", 0).unwrap();
    //         assert_eq!(
    //             res.messages,
    //             vec![
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw721ExecuteMsg::TransferNft {
    //                             recipient: "counterer".to_string(),
    //                             token_id: "58".to_string()
    //                         },
    //                         "nft"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw1155ExecuteMsg::SendFrom {
    //                             to: "counterer".to_string(),
    //                             from: mock_env().contract.address.to_string(),
    //                             token_id: "58".to_string(),
    //                             value: Uint128::from(100u128),
    //                             msg: None
    //                         },
    //                         "cw1155"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw20ExecuteMsg::Transfer {
    //                             recipient: "counterer".to_string(),
    //                             amount: Uint128::from(100u64)
    //                         },
    //                         "token"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(BankMsg::Send {
    //                     to_address: "counterer".to_string(),
    //                     amount: coins(9, "other_token"),
    //                 }),
    //             ]
    //         );

    //         let err = withdraw_helper(deps.as_mut(), "counterer", "fee_contract", 0).unwrap_err();
    //         assert_eq!(err, ContractError::TradeAlreadyWithdrawn {});

    //         let res =
    //             withdraw_aborted_counter_helper(deps.as_mut(), "other_counterer", 0, 0).unwrap();
    //         assert_eq!(
    //             res.messages,
    //             vec![
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw721ExecuteMsg::TransferNft {
    //                             recipient: "other_counterer".to_string(),
    //                             token_id: "58".to_string()
    //                         },
    //                         "other_counter-nft"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw20ExecuteMsg::Transfer {
    //                             recipient: "other_counterer".to_string(),
    //                             amount: Uint128::from(100u64)
    //                         },
    //                         "other_counter-token"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(BankMsg::Send {
    //                     to_address: "other_counterer".to_string(),
    //                     amount: coins(5, "lunas"),
    //                 }),
    //             ]
    //         );

    //         let err = withdraw_aborted_counter_helper(deps.as_mut(), "other_counterer", 0, 0)
    //             .unwrap_err();
    //         assert_eq!(err, ContractError::TradeAlreadyWithdrawn {});
    //     }

    //     #[test]
    //     fn withdraw_cancelled_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());
    //         create_trade_helper(deps.as_mut(), "creator");

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Coin(coin(5, "lunas")),
    //             &coins(5, "lunas"),
    //         )
    //         .unwrap();

    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "other_counter-token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         cancel_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         let res = withdraw_cancelled_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         assert_eq!(
    //             res.messages,
    //             vec![
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw721ExecuteMsg::TransferNft {
    //                             recipient: "creator".to_string(),
    //                             token_id: "58".to_string()
    //                         },
    //                         "nft"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw20ExecuteMsg::Transfer {
    //                             recipient: "creator".to_string(),
    //                             amount: Uint128::from(100u64)
    //                         },
    //                         "token"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(BankMsg::Send {
    //                     to_address: "creator".to_string(),
    //                     amount: coins(5, "lunas"),
    //                 }),
    //             ]
    //         );

    //         let err = withdraw_cancelled_trade_helper(deps.as_mut(), "creator", 0).unwrap_err();
    //         assert_eq!(err, ContractError::TradeAlreadyWithdrawn {});

    //         let res = withdraw_aborted_counter_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         assert_eq!(
    //             res.messages,
    //             vec![SubMsg::new(
    //                 into_cosmos_msg(
    //                     Cw20ExecuteMsg::Transfer {
    //                         recipient: "counterer".to_string(),
    //                         amount: Uint128::from(100u64)
    //                     },
    //                     "other_counter-token"
    //                 )
    //                 .unwrap()
    //             ),]
    //         );

    //         let err =
    //             withdraw_aborted_counter_helper(deps.as_mut(), "counterer", 0, 0).unwrap_err();
    //         assert_eq!(err, ContractError::TradeAlreadyWithdrawn {});
    //     }

    //     #[test]
    //     fn private() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());
    //         create_private_trade_helper(deps.as_mut(), vec!["whitelist".to_string()]);

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_trade_helper(
    //             deps.as_mut(),
    //             "creator",
    //             0,
    //             AssetInfo::Coin(coin(5, "lunas")),
    //             &coins(5, "lunas"),
    //         )
    //         .unwrap();

    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         let err = suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap_err();
    //         assert_eq!(err, ContractError::AddressNotWhitelisted {});

    //         suggest_counter_trade_helper(deps.as_mut(), "whitelist", 0).unwrap();

    //         let err = remove_whitelisted_users(deps.as_mut(), 0, vec!["whitelist".to_string()])
    //             .unwrap_err();
    //         assert_eq!(
    //             err,
    //             ContractError::WrongTradeState {
    //                 state: TradeState::Countered
    //             }
    //         );

    //         let err =
    //             add_whitelisted_users(deps.as_mut(), 0, vec!["whitelist".to_string()]).unwrap_err();
    //         assert_eq!(
    //             err,
    //             ContractError::WrongTradeState {
    //                 state: TradeState::Countered
    //             }
    //         );

    //         create_private_trade_helper(deps.as_mut(), vec!["whitelist".to_string()]);

    //         remove_whitelisted_users(deps.as_mut(), 1, vec!["whitelist".to_string()]).unwrap();
    //         let info = TRADE_INFO.load(&deps.storage, 1_u64).unwrap();
    //         let hash_set = HashSet::new();
    //         assert_eq!(info.whitelisted_users, hash_set);

    //         add_whitelisted_users(
    //             deps.as_mut(),
    //             1,
    //             vec!["whitelist-1".to_string(), "whitelist".to_string()],
    //         )
    //         .unwrap();
    //         add_whitelisted_users(
    //             deps.as_mut(),
    //             1,
    //             vec!["whitelist-2".to_string(), "whitelist".to_string()],
    //         )
    //         .unwrap();
    //         let info = TRADE_INFO.load(&deps.storage, 1_u64).unwrap();

    //         let whitelisted_users = vec![
    //             "whitelist".to_string(),
    //             "whitelist-1".to_string(),
    //             "whitelist-2".to_string(),
    //         ];
    //         let hash_set =
    //             HashSet::from_iter(validate_addresses(&deps.api, &whitelisted_users).unwrap());
    //         assert_eq!(info.whitelisted_users, hash_set);
    //     }
    // }

    // fn suggest_counter_trade_helper(
    //     deps: DepsMut,
    //     counterer: &str,
    //     trade_id: u64,
    // ) -> Result<Response, ContractError> {
    //     let info = mock_info(counterer, &[]);
    //     let env = mock_env();

    //     execute(
    //         deps,
    //         env,
    //         info,
    //         ExecuteMsg::SuggestCounterTrade {
    //             trade_id,
    //             comment: Some("Q".to_string()),
    //         },
    //     )
    // }

    // fn add_asset_to_counter_trade_helper(
    //     deps: DepsMut,
    //     counterer: &str,
    //     trade_id: u64,
    //     counter_id: u64,
    //     asset: AssetInfo,
    //     coins_to_send: &[Coin],
    // ) -> Result<Response, ContractError> {
    //     let info = mock_info(counterer, coins_to_send);
    //     let env = mock_env();

    //     execute(
    //         deps,
    //         env,
    //         info,
    //         ExecuteMsg::AddAsset {
    //             action: AddAssetAction::ToCounterTrade {
    //                 trade_id,
    //                 counter_id,
    //             },
    //             asset,
    //         },
    //     )
    // }

    // fn confirm_counter_trade_helper(
    //     deps: DepsMut,
    //     sender: &str,
    //     trade_id: u64,
    //     counter_id: u64,
    // ) -> Result<Response, ContractError> {
    //     let info = mock_info(sender, &[]);
    //     let env = mock_env();

    //     execute(
    //         deps,
    //         env,
    //         info,
    //         ExecuteMsg::ConfirmCounterTrade {
    //             trade_id,
    //             counter_id: Some(counter_id),
    //         },
    //     )
    // }

    // fn review_counter_trade_helper(
    //     deps: DepsMut,
    //     sender: &str,
    //     trade_id: u64,
    //     counter_id: u64,
    // ) -> Result<Response, ContractError> {
    //     let info = mock_info(sender, &[]);
    //     let env = mock_env();

    //     execute(
    //         deps,
    //         env,
    //         info,
    //         ExecuteMsg::ReviewCounterTrade {
    //             trade_id,
    //             counter_id,
    //             comment: Some("Shit NFT my girl".to_string()),
    //         },
    //     )
    // }

    // fn accept_trade_helper(
    //     deps: DepsMut,
    //     sender: &str,
    //     trade_id: u64,
    //     counter_id: u64,
    // ) -> Result<Response, ContractError> {
    //     let info = mock_info(sender, &[]);
    //     let env = mock_env();

    //     execute(
    //         deps,
    //         env,
    //         info,
    //         ExecuteMsg::AcceptTrade {
    //             trade_id,
    //             counter_id,
    //             comment: Some("You're very kind madam".to_string()),
    //         },
    //     )
    // }

    // fn cancel_trade_helper(
    //     deps: DepsMut,
    //     sender: &str,
    //     trade_id: u64,
    // ) -> Result<Response, ContractError> {
    //     let info = mock_info(sender, &[]);
    //     let env = mock_env();

    //     execute(deps, env, info, ExecuteMsg::CancelTrade { trade_id })
    // }

    // fn cancel_counter_trade_helper(
    //     deps: DepsMut,
    //     sender: &str,
    //     trade_id: u64,
    //     counter_id: u64,
    // ) -> Result<Response, ContractError> {
    //     let info = mock_info(sender, &[]);
    //     let env = mock_env();

    //     execute(
    //         deps,
    //         env,
    //         info,
    //         ExecuteMsg::CancelCounterTrade {
    //             trade_id,
    //             counter_id,
    //         },
    //     )
    // }

    // fn refuse_counter_trade_helper(
    //     deps: DepsMut,
    //     trader: &str,
    //     trade_id: u64,
    //     counter_id: u64,
    // ) -> Result<Response, ContractError> {
    //     let info = mock_info(trader, &[]);
    //     let env = mock_env();

    //     execute(
    //         deps,
    //         env,
    //         info,
    //         ExecuteMsg::RefuseCounterTrade {
    //             trade_id,
    //             counter_id,
    //         },
    //     )
    // }

    // pub mod counter_trade_tests {
    //     use super::*;
    //     use crate::query::{AllTradesResponse, TradeResponse};
    //     use cosmwasm_std::{coin, from_json, Api, SubMsg};
    //     use p2p_trading_export::msg::{
    //         AdditionalTradeInfoResponse, QueryFilters, TradeInfoResponse,
    //     };
    //     use p2p_trading_export::state::{Comment, CounterTradeInfo};

    //     #[test]
    //     fn create_counter_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");

    //         let err = suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap_err();

    //         assert_eq!(err, ContractError::NotCounterable {});

    //         let err = suggest_counter_trade_helper(deps.as_mut(), "counterer", 1).unwrap_err();

    //         assert_eq!(err, ContractError::NotFoundInTradeInfo {});

    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         let res = suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "create_counter_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );
    //         // We need to make sure it is not couterable in case the counter is accepted
    //     }
    //     #[test]
    //     fn create_counter_trade_and_add_funds() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         let res = add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Coin(coin(2, "token")),
    //             &coins(2, "token"),
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "add_asset"),
    //                 Attribute::new("asset_type", "fund"),
    //                 Attribute::new("denom", "token"),
    //                 Attribute::new("amount", "2"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );

    //         let counter_trade_info = load_counter_trade(&deps.storage, 0, 0).unwrap();
    //         assert_eq!(counter_trade_info.state, TradeState::Created);
    //         assert_eq!(
    //             counter_trade_info.associated_assets,
    //             vec![AssetInfo::Coin(coin(2, "token"))]
    //         );
    //     }

    //     #[test]
    //     fn create_counter_trade_and_add_cw20_tokens() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         let res = add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "add_asset"),
    //                 Attribute::new("asset_type", "token"),
    //                 Attribute::new("token", "token"),
    //                 Attribute::new("amount", "100"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );

    //         let err = add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             1,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::NotFoundInCounterTradeInfo {});

    //         // Verifying the state has been changed
    //         let trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(trade_info.state, TradeState::Countered);
    //         assert_eq!(trade_info.associated_assets, vec![]);

    //         let counter_trade_info = load_counter_trade(&deps.storage, 0, 0).unwrap();
    //         assert_eq!(counter_trade_info.state, TradeState::Created);
    //         assert_eq!(
    //             counter_trade_info.associated_assets,
    //             vec![AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::from(100u64)
    //             }),]
    //         );

    //         // This triggers an error, the creator is not the same as the sender
    //         let err = add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "bad_person",
    //             0,
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::CounterTraderNotCreator {});
    //     }

    //     #[test]
    //     fn create_trade_and_add_cw721_tokens() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         let res = add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "add_asset"),
    //                 Attribute::new("asset_type", "NFT"),
    //                 Attribute::new("nft", "nft"),
    //                 Attribute::new("token_id", "58"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );

    //         // Verifying the state has been changed
    //         let trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(trade_info.state, TradeState::Countered);
    //         assert_eq!(trade_info.associated_assets, vec![]);

    //         let counter_trade_info = load_counter_trade(&deps.storage, 0, 0).unwrap();
    //         assert_eq!(counter_trade_info.state, TradeState::Created);
    //         assert_eq!(
    //             counter_trade_info.associated_assets,
    //             vec![AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string()
    //             }),]
    //         );

    //         // This triggers an error, the counter-trade creator is not the same as the sender
    //         let err = add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "bad_person",
    //             0,
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "token".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::CounterTraderNotCreator {});
    //     }

    //     #[test]
    //     fn create_counter_trade_automatic_trade_id() {
    //         let mut deps = mock_dependencies();
    //         let info = mock_info("creator", &[]);
    //         let env = mock_env();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 1).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "creator", 1).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         execute(
    //             deps.as_mut(),
    //             env.clone(),
    //             info,
    //             ExecuteMsg::AddAsset {
    //                 action: AddAssetAction::ToLastCounterTrade { trade_id: 0 },
    //                 asset: AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "cw20".to_string(),
    //                     amount: Uint128::from(100u64),
    //                 }),
    //             },
    //         )
    //         .unwrap();

    //         let info = mock_info("creator", &coins(97u128, "uluna"));
    //         execute(
    //             deps.as_mut(),
    //             env,
    //             info,
    //             ExecuteMsg::AddAsset {
    //                 action: AddAssetAction::ToLastCounterTrade { trade_id: 0 },
    //                 asset: AssetInfo::Coin(coin(97u128, "uluna")),
    //             },
    //         )
    //         .unwrap();

    //         let info = mock_info("creator", &[]);
    //         let env = mock_env();

    //         execute(
    //             deps.as_mut(),
    //             env,
    //             info,
    //             ExecuteMsg::ConfirmCounterTrade {
    //                 trade_id: 0,
    //                 counter_id: None,
    //             },
    //         )
    //         .unwrap();

    //         let trade_info = COUNTER_TRADE_INFO
    //             .load(&deps.storage, (0u64, 0u64))
    //             .unwrap();
    //         assert_eq!(
    //             trade_info.associated_assets,
    //             vec![
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "cw20".to_string(),
    //                     amount: Uint128::from(100u128)
    //                 }),
    //                 AssetInfo::Coin(coin(97, "uluna"))
    //             ]
    //         );
    //         assert_eq!(trade_info.state, TradeState::Published);
    //     }

    //     #[test]
    //     fn create_counter_trade_add_remove_tokens() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft-2".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Coin(coin(100, "luna")),
    //             &coins(100, "luna"),
    //         )
    //         .unwrap();

    //         let res = remove_assets_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             Some(0),
    //             vec![
    //                 (
    //                     0,
    //                     AssetInfo::Cw721Coin(Cw721Coin {
    //                         address: "nft".to_string(),
    //                         token_id: "58".to_string(),
    //                     }),
    //                 ),
    //                 (
    //                     2,
    //                     AssetInfo::Cw20Coin(Cw20Coin {
    //                         address: "token".to_string(),
    //                         amount: Uint128::from(58u64),
    //                     }),
    //                 ),
    //                 (3, AssetInfo::Coin(coin(58, "luna"))),
    //             ],
    //         )
    //         .unwrap();

    //         assert_eq!(res.attributes.len(), 14);
    //         assert_eq!(
    //             res.messages,
    //             vec![
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw721ExecuteMsg::TransferNft {
    //                             recipient: "counterer".to_string(),
    //                             token_id: "58".to_string()
    //                         },
    //                         "nft"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(
    //                     into_cosmos_msg(
    //                         Cw20ExecuteMsg::Transfer {
    //                             recipient: "counterer".to_string(),
    //                             amount: Uint128::from(58u64)
    //                         },
    //                         "token"
    //                     )
    //                     .unwrap()
    //                 ),
    //                 SubMsg::new(BankMsg::Send {
    //                     to_address: "counterer".to_string(),
    //                     amount: coins(58, "luna"),
    //                 })
    //             ]
    //         );

    //         let new_trade_info = load_counter_trade(&deps.storage, 0, 0).unwrap();
    //         assert_eq!(
    //             new_trade_info.associated_assets,
    //             vec![
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     token_id: "58".to_string(),
    //                     address: "nft-2".to_string()
    //                 }),
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     amount: Uint128::from(42u64),
    //                     address: "token".to_string()
    //                 }),
    //                 AssetInfo::Coin(coin(42, "luna"))
    //             ],
    //         );

    //         remove_assets_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             Some(0),
    //             vec![
    //                 (
    //                     1,
    //                     AssetInfo::Cw20Coin(Cw20Coin {
    //                         address: "token".to_string(),
    //                         amount: Uint128::from(42u64),
    //                     }),
    //                 ),
    //                 (2, AssetInfo::Coin(coin(42, "luna"))),
    //             ],
    //         )
    //         .unwrap();

    //         let new_trade_info = load_counter_trade(&deps.storage, 0, 0).unwrap();
    //         assert_eq!(
    //             new_trade_info.associated_assets,
    //             vec![AssetInfo::Cw721Coin(Cw721Coin {
    //                 token_id: "58".to_string(),
    //                 address: "nft-2".to_string()
    //             }),],
    //         );

    //         // This triggers an error, the counterer is not the same as the sender
    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "bad_person",
    //             0,
    //             Some(0),
    //             vec![(
    //                 0,
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     address: "nft-2".to_string(),
    //                     token_id: "58".to_string(),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::CounterTraderNotCreator {});

    //         // This triggers an error, no matching funds were found
    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             Some(0),
    //             vec![(
    //                 1,
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     address: "nft-2".to_string(),
    //                     token_id: "58".to_string(),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 1 });

    //         // This triggers an error, no matching funds were found
    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             Some(0),
    //             vec![(
    //                 0,
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     address: "nft-1".to_string(),
    //                     token_id: "58".to_string(),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 0 });

    //         // This triggers an error, no matching funds were found
    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             Some(0),
    //             vec![(
    //                 0,
    //                 AssetInfo::Cw721Coin(Cw721Coin {
    //                     address: "nft-2".to_string(),
    //                     token_id: "42".to_string(),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 0 });
    //     }

    //     #[test]
    //     fn create_trade_add_remove_tokens_errors() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft-2".to_string(),
    //                 token_id: "58".to_string(),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Cw20Coin(Cw20Coin {
    //                 address: "token".to_string(),
    //                 amount: Uint128::new(100u128),
    //             }),
    //             &[],
    //         )
    //         .unwrap();

    //         add_asset_to_counter_trade_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             0,
    //             AssetInfo::Coin(coin(100, "luna")),
    //             &coins(100, "luna"),
    //         )
    //         .unwrap();

    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             Some(0),
    //             vec![(
    //                 2,
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "token".to_string(),
    //                     amount: Uint128::from(101u64),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(
    //             err,
    //             ContractError::TooMuchWithdrawn {
    //                 address: "token".to_string(),
    //                 wanted: 101,
    //                 available: 100
    //             }
    //         );

    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             Some(0),
    //             vec![(
    //                 0,
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "token".to_string(),
    //                     amount: Uint128::from(101u64),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 0 });

    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             Some(0),
    //             vec![(
    //                 2,
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "wrong-token".to_string(),
    //                     amount: Uint128::from(101u64),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::AssetNotFound { position: 2 });

    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();

    //         let err = remove_assets_helper(
    //             deps.as_mut(),
    //             "counterer",
    //             0,
    //             Some(0),
    //             vec![(
    //                 2,
    //                 AssetInfo::Cw20Coin(Cw20Coin {
    //                     address: "token".to_string(),
    //                     amount: Uint128::from(58u64),
    //                 }),
    //             )],
    //         )
    //         .unwrap_err();

    //         assert_eq!(err, ContractError::CounterTradeAlreadyPublished {});
    //     }

    //     #[test]
    //     fn confirm_counter_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         //Wrong trade id
    //         let err = confirm_counter_trade_helper(deps.as_mut(), "creator", 1, 0).unwrap_err();
    //         assert_eq!(err, ContractError::NotFoundInCounterTradeInfo {});

    //         //Wrong counter id
    //         let err = confirm_counter_trade_helper(deps.as_mut(), "creator", 0, 1).unwrap_err();
    //         assert_eq!(err, ContractError::NotFoundInCounterTradeInfo {});

    //         //Wrong trader
    //         let err = confirm_counter_trade_helper(deps.as_mut(), "bad_person", 0, 0).unwrap_err();
    //         assert_eq!(err, ContractError::CounterTraderNotCreator {});

    //         // This time, it has to work fine
    //         let res = confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "confirm_counter_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );

    //         //Already confirmed
    //         let err = confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap_err();
    //         assert_eq!(
    //             err,
    //             ContractError::CantChangeCounterTradeState {
    //                 from: TradeState::Published,
    //                 to: TradeState::Published
    //             }
    //         );
    //     }

    //     #[test]
    //     fn review_counter_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         //Wrong trade id
    //         let err = review_counter_trade_helper(deps.as_mut(), "creator", 1, 0).unwrap_err();
    //         assert_eq!(err, ContractError::NotFoundInTradeInfo {});

    //         //Wrong counter id
    //         let err = review_counter_trade_helper(deps.as_mut(), "creator", 0, 1).unwrap_err();
    //         assert_eq!(err, ContractError::NotFoundInCounterTradeInfo {});

    //         //Wrong trader
    //         let err = review_counter_trade_helper(deps.as_mut(), "bad_person", 0, 0).unwrap_err();
    //         assert_eq!(err, ContractError::TraderNotCreator {});

    //         let err = review_counter_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap_err();
    //         assert_eq!(
    //             err,
    //             ContractError::CantChangeCounterTradeState {
    //                 from: TradeState::Created,
    //                 to: TradeState::Created
    //             }
    //         );

    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();

    //         // This time, it has to work fine
    //         let res = review_counter_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "review_counter_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );

    //         // Because this was the only counter
    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(new_trade_info.state, TradeState::Countered {});
    //     }

    //     #[test]
    //     fn review_counter_trade_when_accepted() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         accept_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();

    //         let err = review_counter_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap_err();
    //         assert_eq!(err, ContractError::TradeAlreadyAccepted {});
    //     }

    //     #[test]
    //     fn review_counter_trade_when_cancelled() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         cancel_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         let err = review_counter_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap_err();
    //         assert_eq!(err, ContractError::TradeCancelled {});
    //     }

    //     #[test]
    //     fn review_counter_with_multiple() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         // We suggest and confirm one more counter
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 1).unwrap();

    //         // This time, it has to work fine
    //         let res = review_counter_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "review_counter_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );

    //         let new_trade_info = load_trade(&deps.storage, 0).unwrap();
    //         assert_eq!(new_trade_info.state, TradeState::Countered {});
    //     }

    //     #[test]
    //     fn refuse_counter_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         let res = refuse_counter_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "refuse_counter_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );
    //     }

    //     #[test]
    //     fn cancel_counter_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         cancel_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();

    //         let err = accept_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap_err();

    //         assert_eq!(err, ContractError::CantAcceptNotPublishedCounter {});
    //     }

    //     #[test]
    //     fn refuse_counter_trade_with_multiple() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         // We suggest and confirm one more counter
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         // We suggest one more counter
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         let res = refuse_counter_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();
    //         assert_eq!(
    //             res.attributes,
    //             vec![
    //                 Attribute::new("action", "refuse_counter_trade"),
    //                 Attribute::new("trade_id", "0"),
    //                 Attribute::new("counter_id", "0"),
    //                 Attribute::new("trader", "creator"),
    //                 Attribute::new("counter_trader", "counterer"),
    //             ]
    //         );
    //     }

    //     #[test]
    //     fn refuse_accepted_counter_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         accept_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();
    //         let err = refuse_counter_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap_err();
    //         assert_eq!(err, ContractError::TradeAlreadyAccepted {});
    //     }

    //     #[test]
    //     fn cancel_accepted_counter_trade() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();

    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         accept_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();
    //         let err = cancel_trade_helper(deps.as_mut(), "creator", 0).unwrap_err();
    //         assert_eq!(
    //             err,
    //             ContractError::CantChangeTradeState {
    //                 from: TradeState::Accepted,
    //                 to: TradeState::Cancelled
    //             }
    //         );
    //     }

    //     #[test]
    //     fn confirm_counter_trade_after_accepted() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         accept_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();

    //         //Already confirmed
    //         let err = confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap_err();
    //         assert_eq!(
    //             err,
    //             ContractError::CantChangeCounterTradeState {
    //                 from: TradeState::Accepted,
    //                 to: TradeState::Published
    //             }
    //         );
    //     }

    //     #[test]
    //     fn query_trades_by_counterer() {
    //         let mut deps = mock_dependencies();
    //         init_helper(deps.as_mut());

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 0).unwrap();

    //         // When no counter_trades
    //         let env = mock_env();
    //         let res: AllTradesResponse = from_json(
    //             query(
    //                 deps.as_ref(),
    //                 env,
    //                 QueryMsg::GetAllTrades {
    //                     start_after: None,
    //                     limit: None,
    //                     filters: Some(QueryFilters {
    //                         counterer: Some("counterer".to_string()),
    //                         ..QueryFilters::default()
    //                     }),
    //                 },
    //             )
    //             .unwrap(),
    //         )
    //         .unwrap();

    //         assert_eq!(res.trades, vec![]);

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 0).unwrap();
    //         confirm_counter_trade_helper(deps.as_mut(), "counterer", 0, 0).unwrap();
    //         accept_trade_helper(deps.as_mut(), "creator", 0, 0).unwrap();

    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 1).unwrap();
    //         create_trade_helper(deps.as_mut(), "creator");
    //         confirm_trade_helper(deps.as_mut(), "creator", 2).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "bad_person", 1).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "bad_person", 1).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "bad_person", 1).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "bad_person", 1).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "bad_person", 1).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "bad_person", 1).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "bad_person", 1).unwrap();
    //         suggest_counter_trade_helper(deps.as_mut(), "bad_person", 1).unwrap();

    //         suggest_counter_trade_helper(deps.as_mut(), "counterer", 2).unwrap();

    //         let env = mock_env();
    //         let res: AllTradesResponse = from_json(
    //             query(
    //                 deps.as_ref(),
    //                 env,
    //                 QueryMsg::GetAllTrades {
    //                     start_after: None,
    //                     limit: None,
    //                     filters: Some(QueryFilters {
    //                         counterer: Some("counterer".to_string()),
    //                         ..QueryFilters::default()
    //                     }),
    //                 },
    //             )
    //             .unwrap(),
    //         )
    //         .unwrap();

    //         let env = mock_env();
    //         assert_eq!(
    //             res.trades,
    //             vec![
    //                 {
    //                     TradeResponse {
    //                         trade_id: 2,
    //                         counter_id: None,
    //                         trade_info: Some(TradeInfoResponse {
    //                             owner: deps.api.addr_validate("creator").unwrap(),
    //                             last_counter_id: Some(0),
    //                             state: TradeState::Countered,
    //                             additional_info: AdditionalTradeInfoResponse {
    //                                 owner_comment: Some(Comment {
    //                                     comment: "Q".to_string(),
    //                                     time: env.block.time,
    //                                 }),
    //                                 time: env.block.time,
    //                                 ..Default::default()
    //                             },
    //                             ..Default::default()
    //                         }),
    //                     }
    //                 },
    //                 {
    //                     TradeResponse {
    //                         trade_id: 0,
    //                         counter_id: None,
    //                         trade_info: Some(TradeInfoResponse {
    //                             owner: deps.api.addr_validate("creator").unwrap(),
    //                             last_counter_id: Some(3),
    //                             state: TradeState::Accepted,
    //                             accepted_info: Some(CounterTradeInfo {
    //                                 trade_id: 0,
    //                                 counter_id: 0,
    //                             }),
    //                             additional_info: AdditionalTradeInfoResponse {
    //                                 owner_comment: Some(Comment {
    //                                     comment: "Q".to_string(),
    //                                     time: env.block.time,
    //                                 }),
    //                                 time: env.block.time,
    //                                 ..Default::default()
    //                             },
    //                             ..Default::default()
    //                         }),
    //                     }
    //                 }
    //             ]
    //         );
    //     }
    // }

    // #[test]
    // fn create_trade_preview() {
    //     let mut deps = mock_dependencies();
    //     init_helper(deps.as_mut());

    //     create_trade_helper(deps.as_mut(), "creator");

    //     add_asset_to_trade_helper(
    //         deps.as_mut(),
    //         "creator",
    //         0,
    //         AssetInfo::Cw721Coin(Cw721Coin {
    //             address: "nft".to_string(),
    //             token_id: "58".to_string(),
    //         }),
    //         &[],
    //     )
    //     .unwrap();

    //     add_asset_to_trade_helper(
    //         deps.as_mut(),
    //         "creator",
    //         0,
    //         AssetInfo::Cw721Coin(Cw721Coin {
    //             address: "nft".to_string(),
    //             token_id: "59".to_string(),
    //         }),
    //         &[],
    //     )
    //     .unwrap();

    //     execute(
    //         deps.as_mut(),
    //         mock_env(),
    //         mock_info("creator", &[]),
    //         ExecuteMsg::SetTradePreview {
    //             action: AddAssetAction::ToTrade { trade_id: 0 },
    //             asset: AssetInfo::Cw721Coin(Cw721Coin {
    //                 address: "nft".to_string(),
    //                 token_id: "59".to_string(),
    //             }),
    //         },
    //     )
    //     .unwrap();
    // }
}
