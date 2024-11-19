use cosmwasm_std::{
    coin, coins, to_json_binary, Addr, Api, BankMsg, Binary, Coin, Coins, Decimal, Deps, DepsMut,
    Empty, Env, MessageInfo, Response, StdError, StdResult, Storage, Uint128,
};
use sg721_base::msg::CollectionInfoResponse;
use utils::state::AssetInfo;

use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};
use std::iter::FromIterator;

use cw721::Cw721ExecuteMsg;

use cw721_base::Extension;
use p2p_trading_export::msg::into_cosmos_msg;
use p2p_trading_export::state::{
    AdditionalTradeInfo, Comment, CounterTradeInfo, TradeInfo, TradeState,
};
use sg721::{ExecuteMsg as Sg721ExecuteMsg, RoyaltyInfoResponse};

use crate::error::ContractError;
use crate::messages::set_comment;
use crate::state::{
    add_cw721_coin, add_funds, add_sg721_coin, is_trader, load_counter_trade, CONTRACT_INFO,
    COUNTER_TRADE_INFO, LAST_USER_TRADE, TRADE_INFO,
};

/// Query the last trade created by the owner.
/// This should only be used in the same transaction as the trade creation.
/// Otherwise, specify the trade_id directly in the transaction
pub fn get_last_trade_id_created(deps: Deps, by: String) -> Result<u64, ContractError> {
    let owner = deps.api.addr_validate(&by)?;
    LAST_USER_TRADE
        .load(deps.storage, &owner)
        .map_err(|_| ContractError::NotFoundInTradeInfo {})
}

/// Create a new trade and assign it a unique id.
/// Saves this trade as the last one created by a user
pub fn create_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    whitelisted_users: Option<Vec<String>>,
    comment: Option<String>,
) -> Result<Response, ContractError> {
    // We start by creating a new trade_id (simply incremented from the last id)
    let trade_id: u64 = CONTRACT_INFO
        .update(deps.storage, |mut c| -> StdResult<_> {
            c.last_trade_id = c.last_trade_id.map_or(Some(0), |id| Some(id + 1));
            Ok(c)
        })?
        .last_trade_id
        .unwrap(); // This is safe because of the function architecture just there

    TRADE_INFO.update(deps.storage, trade_id, |trade| match trade {
        // If the trade id already exists, the contract is faulty
        // Or an external error happened, or whatever...
        // In that case, we emit an error
        // The priority is : We do not want to overwrite existing data
        Some(_) => Err(ContractError::ExistsInTradeInfo {}),
        None => Ok(TradeInfo {
            owner: info.sender.clone(),
            additional_info: AdditionalTradeInfo {
                time: env.block.time,
                ..Default::default()
            },
            ..Default::default()
        }),
    })?;

    // We add whitelisted addresses
    if let Some(whitelist) = whitelisted_users {
        add_whitelisted_users(
            deps.storage,
            deps.api,
            env.clone(),
            info.clone(),
            trade_id,
            whitelist,
        )?;
    }

    // We also set the last trade_id created to this id
    LAST_USER_TRADE.save(deps.storage, &info.sender, &trade_id)?;

    // And the eventual comment sent along with the transaction
    if let Some(comment) = comment {
        set_comment(deps, env, info.clone(), trade_id, None, comment)?;
    }

    Ok(Response::new()
        .add_attribute("action", "create_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// We verify the trader is indeed the sender and the trade can be modified
pub fn can_modify_trade(
    storage: &dyn Storage,
    trader: Addr,
    trade_id: u64,
) -> Result<TradeInfo, ContractError> {
    let trade_info = is_trader(storage, &trader, trade_id)?;
    // 3.
    if trade_info.state != TradeState::Created {
        return Err(ContractError::WrongTradeState {
            state: trade_info.state,
        });
    }
    Ok(trade_info)
}

pub fn trade_id_or_last(
    deps: Deps,
    trader: Addr,
    trade_id: Option<u64>,
) -> Result<u64, ContractError> {
    match trade_id {
        Some(trade_id) => Ok(trade_id),
        None => get_last_trade_id_created(deps, trader.to_string()),
    }
}

/// We prepare the info before asset addition
/// 1. If the trade_id is not specified, we get the last trade_id created by the sender
/// 2. We verify the trade can be modified
pub fn prepare_trade_modification(
    deps: Deps,
    trader: Addr,
    trade_id: Option<u64>,
) -> Result<(u64, TradeInfo), ContractError> {
    let trade_id = trade_id_or_last(deps, trader.clone(), trade_id)?;
    let trade_info = can_modify_trade(deps.storage, trader, trade_id)?;
    Ok((trade_id, trade_info))
}

/// We prepare the info before asset addition
/// 1. If the trade_id is not specified, we get the last trade_id created by the sender
/// 2. We verify the trade can be modified
pub fn prepare_harmless_trade_modifications(
    deps: Deps,
    trader: Addr,
    trade_id: Option<u64>,
) -> Result<(u64, TradeInfo), ContractError> {
    let trade_id = trade_id_or_last(deps, trader.clone(), trade_id)?;
    let trade_info = is_trader(deps.storage, &trader, trade_id)?;
    Ok((trade_id, trade_info))
}

pub fn _create_receive_asset_messages(
    env: Env,
    _info: MessageInfo,
    asset: AssetInfo,
) -> Result<Response, ContractError> {
    Ok(match asset {
        AssetInfo::Coin(coin) => Response::new()
            .add_attribute("action", "add_asset")
            .add_attribute("asset_type", "fund")
            .add_attribute("denom", coin.denom)
            .add_attribute("amount", coin.amount),

        AssetInfo::Cw721Coin(token) => {
            let message = Cw721ExecuteMsg::TransferNft {
                recipient: env.contract.address.into(),
                token_id: token.token_id.clone(),
            };

            Response::new()
                .add_message(into_cosmos_msg(message, token.address.clone())?)
                .add_attribute("action", "add_asset")
                .add_attribute("asset_type", "NFT")
                .add_attribute("nft", token.address)
                .add_attribute("token_id", token.token_id)
        }
        AssetInfo::Sg721Token(token) => {
            let message = Sg721ExecuteMsg::<Extension, Empty>::TransferNft {
                recipient: env.contract.address.into(),
                token_id: token.token_id.clone(),
            };

            Response::new()
                .add_message(into_cosmos_msg(message, token.address.clone())?)
                .add_attribute("action", "add_asset")
                .add_attribute("asset_type", "NFT")
                .add_attribute("nft", token.address)
                .add_attribute("token_id", token.token_id)
        }
    })
}

/// Adding a new asset to a trade.
/// This function handles 4 different types of assets
pub fn add_asset_to_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: Option<u64>,
    asset: AssetInfo,
) -> Result<Response, ContractError> {
    let (trade_id, _trade_info) =
        prepare_trade_modification(deps.as_ref(), info.sender.clone(), trade_id)?;

    match asset.clone() {
        AssetInfo::Coin(coin) => {
            TRADE_INFO.update(deps.storage, trade_id, add_funds(coin, info.funds.clone()))
        }
        AssetInfo::Cw721Coin(token) => TRADE_INFO.update(
            deps.storage,
            trade_id,
            add_cw721_coin(token.address.clone(), token.token_id),
        ),
        AssetInfo::Sg721Token(token) => TRADE_INFO.update(
            deps.storage,
            trade_id,
            add_sg721_coin(token.address.clone(), token.token_id),
        ),
    }?;

    // Now we need to transfer the token
    Ok(_create_receive_asset_messages(env, info.clone(), asset)?
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Allows to withdraw assets while creating a trade
/// The assets vector specifies a position (u16) and an asset Info (AssetInfo)
/// The u16 is simply the position of the asset in the associated_assets vector of the TradeInfo struct
/// This position is accessible when querying the TradeInfo.
/// We made this choice to avoid looping over assets when withdrawing unique assets.
/// This allows users to withdraw single assets without a risk of running out of gas.
pub fn withdraw_trade_assets_while_creating(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
    assets: Vec<(u16, AssetInfo)>,
) -> Result<Response, ContractError> {
    // We verify the sender is allowed to withdraw funds
    let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;
    if trade_info.state != TradeState::Created {
        return Err(ContractError::TradeAlreadyPublished {});
    }

    // We verify the assets the users want to withdraw are indeed in the transaction
    _are_assets_in_trade(&trade_info, &assets)?;

    // We withdraw the assets
    _try_withdraw_assets_unsafe(&mut trade_info, &assets)?;

    // We make sure the asset was not the advertised asset
    // For CW721, we match the whole assetInfo
    // For Cw1155 we only match the address and the token_id
    if let Some(preview) = trade_info.additional_info.trade_preview.clone() {
        match preview {
            AssetInfo::Cw721Coin(_) => {
                if assets.iter().any(|r| r.1 == preview) {
                    trade_info.additional_info.trade_preview = None;
                }
            }
            AssetInfo::Sg721Token(_) => {
                if assets.iter().any(|r| r.1 == preview) {
                    trade_info.additional_info.trade_preview = None;
                }
            }
            _ => {}
        }
    }

    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    // We send the assets back to the sender
    let res = _create_withdraw_messages_unsafe(
        deps.as_ref(),
        &info.sender,
        &assets.iter().map(|x| x.1.clone()).collect(),
        None,
        None,
    )?;
    Ok(res
        .add_attribute("action", "remove_from_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", trade_info.owner))
}

/// Helper function to verify the specified `assets` indeed exist in the trade and have the right position specified
pub fn _are_assets_in_trade(
    trade_info: &TradeInfo,
    assets: &[(u16, AssetInfo)],
) -> Result<(), ContractError> {
    // We first treat the assets
    for (position, asset) in assets {
        let position: usize = (*position).into();

        if position >= trade_info.associated_assets.len() {
            return Err(ContractError::AssetNotFound { position });
        }
        let asset_info: AssetInfo = trade_info.associated_assets[position].clone();
        match asset_info {
            AssetInfo::Coin(fund_info) => {
                // We check the fund is the one we want
                if let AssetInfo::Coin(fund) = asset {
                    // We verify the sent information matches the saved fund
                    if fund_info.denom != fund.denom {
                        return Err(ContractError::AssetNotFound { position });
                    }
                    if fund_info.amount < fund.amount {
                        return Err(ContractError::TooMuchWithdrawn {
                            address: fund_info.denom,
                            wanted: fund.amount.u128(),
                            available: fund_info.amount.u128(),
                        });
                    }
                }
            }

            AssetInfo::Cw721Coin(nft_info) => {
                // We check the token is the one we want
                if let AssetInfo::Cw721Coin(nft) = asset {
                    // We verify the sent information matches the saved nft
                    if nft_info.address != nft.address {
                        return Err(ContractError::AssetNotFound { position });
                    }
                    if nft_info.token_id != nft.token_id {
                        return Err(ContractError::AssetNotFound { position });
                    }
                } else {
                    return Err(ContractError::AssetNotFound { position });
                }
            }
            AssetInfo::Sg721Token(nft_info) => {
                // We check the token is the one we want
                if let AssetInfo::Sg721Token(nft) = asset {
                    // We verify the sent information matches the saved nft
                    if nft_info.address != nft.address {
                        return Err(ContractError::AssetNotFound { position });
                    }
                    if nft_info.token_id != nft.token_id {
                        return Err(ContractError::AssetNotFound { position });
                    }
                } else {
                    return Err(ContractError::AssetNotFound { position });
                }
            }
        }
    }

    Ok(())
}

/// Helper function to remove withdrawn assets from the trade in the internal data_structure
pub fn _try_withdraw_assets_unsafe(
    trade_info: &mut TradeInfo,
    assets: &[(u16, AssetInfo)],
) -> Result<(), ContractError> {
    for (position, asset) in assets {
        let position: usize = (*position).into();
        let asset_info = trade_info.associated_assets[position].clone();
        match asset_info {
            AssetInfo::Coin(mut fund_info) => {
                if let AssetInfo::Coin(fund) = asset {
                    // If everything is in order, we remove the coin from the trade
                    fund_info.amount = fund_info
                        .amount
                        .checked_sub(fund.amount)
                        .map_err(ContractError::Overflow)?;
                    trade_info.associated_assets[position] = AssetInfo::Coin(fund_info);
                }
            }
            AssetInfo::Cw721Coin(mut nft_info) => {
                if let AssetInfo::Cw721Coin(_) = asset {
                    nft_info.address = "".to_string();
                    trade_info.associated_assets[position] = AssetInfo::Cw721Coin(nft_info);
                }
            }
            AssetInfo::Sg721Token(mut nft_info) => {
                if let AssetInfo::Cw721Coin(_) = asset {
                    nft_info.address = "".to_string();
                    trade_info.associated_assets[position] = AssetInfo::Sg721Token(nft_info);
                }
            }
        }
    }

    // Then we remove empty assets from the trade
    trade_info.associated_assets.retain(|asset| match asset {
        AssetInfo::Coin(fund) => fund.amount != Uint128::zero(),
        AssetInfo::Cw721Coin(nft) => !nft.address.is_empty(),
        AssetInfo::Sg721Token(nft) => !nft.address.is_empty(),
    });

    Ok(())
}

/// Helper function to create withdraw messages based on a slice of assets.
/// This function doesn't do any checks and must be used with caution
/// We must always verify the sender has the right to withdraw before calling this function
#[allow(clippy::ptr_arg)]
pub fn _create_withdraw_messages_unsafe(
    deps: Deps,
    recipient: &Addr,
    assets: &Vec<AssetInfo>,
    royalties: Option<Vec<RoyaltyInfoResponse>>,
    fund_fee: Option<Decimal>,
) -> Result<Response, ContractError> {
    let mut res = Response::new();

    // First the assets
    for asset in assets {
        match asset {
            AssetInfo::Coin(fund) => {
                let (royalty_msgs, funds_after_royalties) = if let Some(royalties) = &royalties {
                    let nb_royalties = royalties.len() as u128;

                    if nb_royalties == 0 {
                        (vec![], fund.clone())
                    } else {
                        let mut total_amount = fund.amount;
                        let amount_per_nft = fund.amount / Uint128::from(nb_royalties);

                        let msgs = royalties
                            .iter()
                            .filter_map(|r| {
                                let amount = r.share * amount_per_nft;
                                total_amount -= amount;
                                if amount.is_zero() {
                                    None
                                } else {
                                    Some(BankMsg::Send {
                                        to_address: r.payment_address.clone(),
                                        amount: coins(amount.u128(), fund.denom.clone()),
                                    })
                                }
                            })
                            .collect::<Vec<_>>();

                        (msgs, coin(total_amount.u128(), fund.denom.clone()))
                    }
                } else {
                    (vec![], fund.clone())
                };

                let (fee_msgs, funds_for_recipient) = if let Some(fund_fee) = fund_fee {
                    let fee_amount = funds_after_royalties.amount * fund_fee;
                    let fee_denom = funds_after_royalties.denom.clone();
                    let remaining_amount = funds_after_royalties.amount - fee_amount;

                    (
                        if fee_amount.is_zero() {
                            None
                        } else {
                            Some(BankMsg::Send {
                                to_address: CONTRACT_INFO.load(deps.storage)?.treasury.to_string(),
                                amount: coins(fee_amount.u128(), fee_denom.clone()),
                            })
                        },
                        coin(remaining_amount.u128(), fee_denom),
                    )
                } else {
                    (None, funds_after_royalties)
                };

                let withdraw_message = BankMsg::Send {
                    to_address: recipient.to_string(),
                    amount: vec![funds_for_recipient.clone()],
                };

                res = res
                    .add_messages(royalty_msgs)
                    .add_messages(fee_msgs)
                    .add_message(withdraw_message)
                    .add_attribute("asset_type", "fund")
                    .add_attribute("denom", fund.denom.clone())
                    .add_attribute("amount", fund.amount);
            }
            AssetInfo::Cw721Coin(nft) => {
                let message = Cw721ExecuteMsg::TransferNft {
                    recipient: recipient.to_string(),
                    token_id: nft.token_id.clone(),
                };
                res = res
                    .add_message(into_cosmos_msg(message, nft.address.clone())?)
                    .add_attribute("asset_type", "NFT")
                    .add_attribute("nft", nft.address.clone())
                    .add_attribute("token_id", nft.token_id.clone());
            }
            AssetInfo::Sg721Token(nft) => {
                let message = Sg721ExecuteMsg::<Extension, Empty>::TransferNft {
                    recipient: recipient.to_string(),
                    token_id: nft.token_id.clone(),
                };
                res = res
                    .add_message(into_cosmos_msg(message, nft.address.clone())?)
                    .add_attribute("asset_type", "NFT")
                    .add_attribute("nft", nft.address.clone())
                    .add_attribute("token_id", nft.token_id.clone());
            }
        }
    }

    Ok(res)
}

/// Check the assets are not already withdrawn and then creates the withdraw messages
pub fn check_and_create_withdraw_messages(
    deps: Deps,
    recipient: &Addr,
    trade_info: &TradeInfo,
    royalties: Option<Vec<RoyaltyInfoResponse>>,
    fund_fee: Option<Decimal>,
) -> Result<Response, ContractError> {
    if trade_info.assets_withdrawn {
        return Err(ContractError::TradeAlreadyWithdrawn {});
    }

    _create_withdraw_messages_unsafe(
        deps,
        recipient,
        &trade_info.associated_assets,
        royalties,
        fund_fee,
    )
}

/// Gathers all the percentages needed to compute the Atlas Fee as well as the Royalties fee
pub fn gather_royalties(
    deps: Deps,
    trade_info: &TradeInfo,
) -> Result<Vec<RoyaltyInfoResponse>, ContractError> {
    // For all assets in the trade_info, we gather the collection info
    let all_royalties = trade_info
        .associated_assets
        .iter()
        .filter_map(|asset| match asset {
            AssetInfo::Cw721Coin(_) => None,
            AssetInfo::Coin(_) => None,
            AssetInfo::Sg721Token(token) => {
                let collection_info: Result<CollectionInfoResponse, _> =
                    deps.querier.query_wasm_smart(
                        token.address.clone(),
                        &sg721_base::msg::QueryMsg::CollectionInfo {},
                    );

                match collection_info {
                    Ok(collection_info) => collection_info.royalty_info,
                    Err(_) => None,
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(all_royalties)
}

/// Helper to validate a slice of addresses
pub fn validate_addresses(api: &dyn Api, whitelisted_users: &[String]) -> StdResult<Vec<Addr>> {
    whitelisted_users
        .iter()
        .map(|x| api.addr_validate(x))
        .collect()
}

/// Add new whitelisted users to a trade
pub fn add_whitelisted_users(
    storage: &mut dyn Storage,
    api: &dyn Api,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
    whitelisted_users: Vec<String>,
) -> Result<Response, ContractError> {
    // We verify the trade can be modified
    let mut trade_info = can_modify_trade(storage, info.sender.clone(), trade_id)?;
    // We modify the whitelist
    let hash_set: HashSet<Addr> = HashSet::from_iter(validate_addresses(api, &whitelisted_users)?);
    trade_info.whitelisted_users = trade_info
        .whitelisted_users
        .union(&hash_set)
        .cloned()
        .collect();
    TRADE_INFO.save(storage, trade_id, &trade_info)?;

    let mut users_attribute = whitelisted_users.join(",");
    if users_attribute.is_empty() {
        users_attribute = "None".to_string()
    }

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "whitelisted_users")
        .add_attribute("operation_type", "add")
        .add_attribute("value", users_attribute)
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Remove whitelisted users from a trade
pub fn remove_whitelisted_users(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
    whitelisted_users: Vec<String>,
) -> Result<Response, ContractError> {
    // We verify the trade can be modified
    let mut trade_info = can_modify_trade(deps.storage, info.sender.clone(), trade_id)?;
    // We modify the whitelist
    let valid_whitelisted_users = validate_addresses(deps.api, &whitelisted_users)?;
    for user in &valid_whitelisted_users {
        trade_info.whitelisted_users.remove(user);
    }
    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "whitelisted_users")
        .add_attribute("operation_type", "remove")
        .add_attribute("value", whitelisted_users.join(","))
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Add wanted nfts (only informational) to a trade
pub fn add_nfts_wanted(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: Option<u64>,
    nfts_wanted: Vec<String>,
) -> Result<Response, ContractError> {
    // We verify the trade can be modified
    let (trade_id, mut trade_info) =
        prepare_harmless_trade_modifications(deps.as_ref(), info.sender.clone(), trade_id)?;
    // We modify the nfts wanted
    let hash_set: HashSet<Addr> = HashSet::from_iter(validate_addresses(deps.api, &nfts_wanted)?);
    trade_info.additional_info.nfts_wanted = trade_info
        .additional_info
        .nfts_wanted
        .union(&hash_set)
        .cloned()
        .collect();

    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "nfts_wanted")
        .add_attribute("operation_type", "add")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Remove wanted nfts (only informational) from a trade
pub fn remove_nfts_wanted(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
    nfts_wanted: Vec<String>,
) -> Result<Response, ContractError> {
    // We verify the caller of the function is the trader
    let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;

    // We modify the whitelist
    let valid_nfts_wanted = validate_addresses(deps.api, &nfts_wanted)?;
    for nft in &valid_nfts_wanted {
        trade_info.additional_info.nfts_wanted.remove(nft);
    }
    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "nfts_wanted")
        .add_attribute("operation_type", "remove")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Set wanted nfts (only informational) to a trade
pub fn set_nfts_wanted(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: Option<u64>,
    nfts_wanted: Vec<String>,
) -> Result<Response, ContractError> {
    // We verify the trade can be modified
    let (trade_id, mut trade_info) =
        prepare_harmless_trade_modifications(deps.as_ref(), info.sender.clone(), trade_id)?;
    // We modify the nfts wanted
    trade_info.additional_info.nfts_wanted =
        HashSet::from_iter(validate_addresses(deps.api, &nfts_wanted)?);

    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "nfts_wanted")
        .add_attribute("operation_type", "set")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Flush wanted nfts (only informational) from a trade
pub fn flush_nfts_wanted(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
) -> Result<Response, ContractError> {
    // We verify the caller of the function is the trader
    let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;

    // We modify the whitelist
    trade_info.additional_info.nfts_wanted = HashSet::new();
    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "nfts_wanted")
        .add_attribute("operation_type", "flush")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Add wanted nfts (only informational) to a trade
pub fn add_tokens_wanted(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: Option<u64>,
    tokens_wanted: Vec<Coin>,
) -> Result<Response, ContractError> {
    // We verify the trade can be modified
    let (trade_id, mut trade_info) =
        prepare_harmless_trade_modifications(deps.as_ref(), info.sender.clone(), trade_id)?;

    let mut old_tokens_wanted: Coins = trade_info.additional_info.tokens_wanted.try_into()?;

    for token in tokens_wanted {
        old_tokens_wanted.add(token)?;
    }

    trade_info.additional_info.tokens_wanted = old_tokens_wanted.into();

    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "tokens_wanted")
        .add_attribute("operation_type", "add")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Remove wanted tokens (only informational) from a trade
pub fn remove_tokens_wanted(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
    tokens_wanted: Vec<Coin>,
) -> Result<Response, ContractError> {
    // We verify the trade can be modified
    let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;
    // We modify the whitelist

    let mut old_tokens_wanted: Coins = trade_info.additional_info.tokens_wanted.try_into()?;

    for token in tokens_wanted {
        old_tokens_wanted.sub(token)?;
    }

    trade_info.additional_info.tokens_wanted = old_tokens_wanted.into();

    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "tokens_wanted")
        .add_attribute("operation_type", "remove")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Set wanted tokens (only informational) to a trade
pub fn set_tokens_wanted(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: Option<u64>,
    tokens_wanted: Vec<Coin>,
) -> Result<Response, ContractError> {
    // We verify the trade can be modified
    let (trade_id, mut trade_info) =
        prepare_harmless_trade_modifications(deps.as_ref(), info.sender.clone(), trade_id)?;

    // We modify the coins wanted
    let validated_coins: Coins = tokens_wanted.try_into()?;
    trade_info.additional_info.tokens_wanted = validated_coins.into();

    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "tokens_wanted")
        .add_attribute("operation_type", "set")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Remove wanted tokens (only informational) from a trade
pub fn flush_tokens_wanted(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
) -> Result<Response, ContractError> {
    // We verify the trade can be modified
    let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;
    // We flush the wanted tokens
    trade_info.additional_info.tokens_wanted = vec![];

    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("name", "tokens_wanted")
        .add_attribute("operation_type", "remove")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Confirm (and publish) a trade when creation is finished
pub fn confirm_trade(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: Option<u64>,
) -> Result<Response, ContractError> {
    // We verify the trade can be published
    let trade_id = trade_id_or_last(deps.as_ref(), info.sender.clone(), trade_id)?;
    let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;

    // We ensure the current trade state allows confirmation
    if trade_info.state != TradeState::Created {
        return Err(ContractError::CantChangeTradeState {
            from: trade_info.state,
            to: TradeState::Published,
        });
    }

    // We set the state as published
    trade_info.state = TradeState::Published;
    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "confirm_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", info.sender))
}

/// Accept a counter trade
pub fn accept_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: u64,
    comment: Option<String>,
) -> Result<Response, ContractError> {
    // Only the initial trader can accept a trade
    let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;

    // We check the counter trade exists
    let mut counter_info = load_counter_trade(deps.storage, trade_id, counter_id)?;

    // We check we can accept the trade
    if trade_info.state != TradeState::Countered {
        // TARPAULIN : This code does not seem to be reachable
        return Err(ContractError::CantChangeTradeState {
            from: trade_info.state,
            to: TradeState::Accepted,
        });
    }
    // We check this specific counter trade can be accepted
    if counter_info.state != TradeState::Published {
        return Err(ContractError::CantAcceptNotPublishedCounter {});
    }

    // We accept the trade
    // We update the trade accepted info to make indexing easier
    let accepted_info = CounterTradeInfo {
        trade_id,
        counter_id,
    };
    trade_info.state = TradeState::Accepted;
    trade_info.accepted_info = Some(accepted_info);
    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    // We update the counter info comment and state
    counter_info.additional_info.trader_comment = comment.map(|comment| Comment {
        time: env.block.time,
        comment,
    });
    counter_info.state = TradeState::Accepted;
    COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &counter_info)?;

    Ok(Response::new()
        .add_attribute("action", "accept_counter_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", counter_info.owner))
}

/// Refuse a counter trade
/// This function is only informational and not needed if the user doesn't deem it necessary
pub fn refuse_counter_trade(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
    counter_id: u64,
) -> Result<Response, ContractError> {
    // Only the initial trader can refuse a trade
    let trade_info = is_trader(deps.storage, &info.sender, trade_id)?;
    // We check the counter trade exists
    let mut counter_info = load_counter_trade(deps.storage, trade_id, counter_id)?;

    if trade_info.state == TradeState::Accepted {
        return Err(ContractError::TradeAlreadyAccepted {});
    }
    if trade_info.state == TradeState::Cancelled {
        return Err(ContractError::TradeCancelled {});
    }
    counter_info.state = TradeState::Refused;
    COUNTER_TRADE_INFO.save(deps.storage, (trade_id, counter_id), &counter_info)?;

    Ok(Response::new()
        .add_attribute("action", "refuse_counter_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("counter_id", counter_id.to_string())
        .add_attribute("trader", trade_info.owner)
        .add_attribute("counter_trader", counter_info.owner))
}

/// Cancel a trade
/// The trade isn't modifiable, but the funds are withdrawnable after this call.
pub fn cancel_trade(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
) -> Result<Response, ContractError> {
    // Only the initial trader can cancel the trade
    let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;

    // We can't cancel an accepted trade
    if trade_info.state == TradeState::Accepted {
        return Err(ContractError::CantChangeTradeState {
            from: trade_info.state,
            to: TradeState::Cancelled,
        });
    }

    // We change the trade state
    trade_info.state = TradeState::Cancelled;
    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(Response::new()
        .add_attribute("action", "cancel_trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", trade_info.owner))
}

/// Withdraw all assets from a created (not published yet) or cancelled trade
/// If the trade is only in the created state, it is automatically cancelled before withdrawing assets
pub fn withdraw_all_from_trade(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trade_id: u64,
) -> Result<Response, ContractError> {
    // We load the trade and verify it has the right trader
    let mut trade_info = is_trader(deps.storage, &info.sender, trade_id)?;

    // If the trade was just created, we cancel it on the spot
    if trade_info.state == TradeState::Created {
        trade_info.state = TradeState::Cancelled;
    }
    // This function is only callable if the trade is cancelled
    if trade_info.state != TradeState::Cancelled {
        return Err(ContractError::TradeNotCancelled {});
    }

    let res =
        check_and_create_withdraw_messages(deps.as_ref(), &info.sender, &trade_info, None, None)?;
    trade_info.assets_withdrawn = true;
    TRADE_INFO.save(deps.storage, trade_id, &trade_info)?;

    Ok(res
        .add_attribute("action", "withdraw_all_funds")
        .add_attribute("type", "trade")
        .add_attribute("trade_id", trade_id.to_string())
        .add_attribute("trader", trade_info.owner))
}
