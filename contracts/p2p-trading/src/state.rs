use cw_storage_plus::{Item, Map};

use cosmwasm_std::{Addr, Coin, StdError, StdResult, Storage, Uint128};

use crate::error::ContractError;
use p2p_trading_export::state::{
    AssetInfo, ContractInfo, Cw1155Coin, Cw20Coin, Cw721Coin, TradeInfo, TradeState,
};

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");

pub const TRADE_INFO: Map<u64, TradeInfo> = Map::new("trade_info");

pub const COUNTER_TRADE_INFO: Map<(u64, u64), TradeInfo> = Map::new("counter_trade_info");

pub const LAST_USER_TRADE: Map<&Addr, u64> = Map::new("last_user_trade");

pub const LAST_USER_COUNTER_TRADE: Map<(&Addr, u64), u64> = Map::new("last_user_counter_trade");

pub fn add_funds(
    fund: Coin,
    info_funds: Vec<Coin>,
) -> impl FnOnce(Option<TradeInfo>) -> Result<TradeInfo, ContractError> {
    move |d: Option<TradeInfo>| -> Result<TradeInfo, ContractError> {
        match d {
            Some(mut trade) => {
                // We check the sent funds are with the right format
                if info_funds.len() != 1 || fund != info_funds[0] {
                    return Err(ContractError::Std(StdError::generic_err(
                        "Funds sent do not match message AssetInfo",
                    )));
                }
                let existing_denom = trade.associated_assets.iter_mut().find(|c| match c {
                    AssetInfo::Coin(x) => x.denom == fund.denom,
                    _ => false,
                });

                if let Some(existing_fund) = existing_denom {
                    let current_amount = match existing_fund {
                        AssetInfo::Coin(x) => x.amount,
                        _ => Uint128::zero(),
                    };
                    *existing_fund = AssetInfo::Coin(Coin {
                        denom: fund.denom,
                        amount: current_amount + fund.amount,
                    });
                } else {
                    trade.associated_assets.push(AssetInfo::Coin(fund));
                }
                Ok(trade)
            }
            //TARPAULIN : Unreachable in current code state
            None => Err(ContractError::NotFoundInTradeInfo {}),
        }
    }
}

pub fn add_cw20_coin(
    address: String,
    sent_amount: Uint128,
) -> impl FnOnce(Option<TradeInfo>) -> Result<TradeInfo, ContractError> {
    move |d: Option<TradeInfo>| -> Result<TradeInfo, ContractError> {
        match d {
            Some(mut trade) => {
                let existing_token = trade.associated_assets.iter_mut().find(|c| match c {
                    AssetInfo::Cw20Coin(x) => x.address == address,
                    _ => false,
                });
                if let Some(existing_token) = existing_token {
                    let current_amount = match existing_token {
                        AssetInfo::Cw20Coin(x) => x.amount,
                        _ => Uint128::zero(),
                    };
                    *existing_token = AssetInfo::Cw20Coin(Cw20Coin {
                        address,
                        amount: current_amount + sent_amount,
                    })
                } else {
                    trade.associated_assets.push(AssetInfo::Cw20Coin(Cw20Coin {
                        address,
                        amount: sent_amount,
                    }))
                }

                Ok(trade)
            }
            //TARPAULIN : Unreachable in current code state
            None => Err(ContractError::NotFoundInTradeInfo {}),
        }
    }
}

pub fn add_cw721_coin(
    address: String,
    token_id: String,
) -> impl FnOnce(Option<TradeInfo>) -> Result<TradeInfo, ContractError> {
    move |d: Option<TradeInfo>| -> Result<TradeInfo, ContractError> {
        match d {
            Some(mut one) => {
                one.associated_assets
                    .push(AssetInfo::Cw721Coin(Cw721Coin { address, token_id }));
                Ok(one)
            }
            //TARPAULIN : Unreachable in current code state
            None => Err(ContractError::NotFoundInTradeInfo {}),
        }
    }
}

pub fn add_cw1155_coin(
    address: String,
    token_id: String,
    value: Uint128,
) -> impl FnOnce(Option<TradeInfo>) -> Result<TradeInfo, ContractError> {
    move |d: Option<TradeInfo>| -> Result<TradeInfo, ContractError> {
        match d {
            Some(mut trade) => {
                let existing_token = trade.associated_assets.iter_mut().find(|c| match c {
                    AssetInfo::Cw1155Coin(x) => x.address == address && x.token_id == token_id,
                    _ => false,
                });
                if let Some(existing_token) = existing_token {
                    let current_value = match existing_token {
                        AssetInfo::Cw1155Coin(x) => x.value,
                        _ => Uint128::zero(),
                    };
                    *existing_token = AssetInfo::Cw1155Coin(Cw1155Coin {
                        address,
                        token_id,
                        value: current_value + value,
                    })
                } else {
                    trade
                        .associated_assets
                        .push(AssetInfo::Cw1155Coin(Cw1155Coin {
                            address,
                            token_id,
                            value,
                        }))
                }

                Ok(trade)
            }
            //TARPAULIN : Unreachable in current code state
            None => Err(ContractError::NotFoundInTradeInfo {}),
        }
    }
}

pub fn is_owner(storage: &dyn Storage, sender: Addr) -> Result<ContractInfo, ContractError> {
    let contract_info = CONTRACT_INFO.load(storage)?;
    if sender == contract_info.owner {
        Ok(contract_info)
    } else {
        Err(ContractError::Unauthorized {})
    }
}

pub fn is_fee_contract(storage: &dyn Storage, sender: Addr) -> Result<(), ContractError> {
    let contract_info = CONTRACT_INFO.load(storage)?;
    if let Some(fee_contract) = contract_info.fee_contract {
        if sender == fee_contract {
            Ok(())
        } else {
            Err(ContractError::Unauthorized {})
        }
    } else {
        Err(ContractError::Unauthorized {})
    }
}

pub fn is_trader(
    storage: &dyn Storage,
    sender: &Addr,
    trade_id: u64,
) -> Result<TradeInfo, ContractError> {
    let trade = load_trade(storage, trade_id)?;

    if trade.owner == sender.clone() {
        Ok(trade)
    } else {
        Err(ContractError::TraderNotCreator {})
    }
}

pub fn is_counter_trader(
    storage: &dyn Storage,
    sender: &Addr,
    trade_id: u64,
    counter_id: u64,
) -> Result<TradeInfo, ContractError> {
    let trade = load_counter_trade(storage, trade_id, counter_id)?;

    if trade.owner == sender.clone() {
        Ok(trade)
    } else {
        Err(ContractError::CounterTraderNotCreator {})
    }
}

pub fn get_actual_counter_state(
    storage: &dyn Storage,
    trade_id: u64,
    counter_info: &mut TradeInfo,
) -> StdResult<()> {
    let trade_info = TRADE_INFO.load(storage, trade_id)?;

    match trade_info.state {
        TradeState::Refused => counter_info.state = TradeState::Cancelled,
        TradeState::Cancelled => counter_info.state = TradeState::Cancelled,
        TradeState::Accepted => match counter_info.state {
            TradeState::Accepted => {}
            _ => counter_info.state = TradeState::Refused,
        },
        _ => {}
    }
    Ok(())
}

pub fn load_counter_trade(
    storage: &dyn Storage,
    trade_id: u64,
    counter_id: u64,
) -> Result<TradeInfo, ContractError> {
    let mut counter = COUNTER_TRADE_INFO
        .load(storage, (trade_id, counter_id))
        .map_err(|_| ContractError::NotFoundInCounterTradeInfo {})?;

    get_actual_counter_state(storage, trade_id, &mut counter)?;

    Ok(counter)
}

pub fn load_trade(storage: &dyn Storage, trade_id: u64) -> Result<TradeInfo, ContractError> {
    TRADE_INFO
        .load(storage, trade_id)
        .map_err(|_| ContractError::NotFoundInTradeInfo {})
}

pub fn can_suggest_counter_trade(
    storage: &dyn Storage,
    trade_id: u64,
    sender: &Addr,
) -> Result<TradeInfo, ContractError> {
    if let Ok(Some(trade)) = TRADE_INFO.may_load(storage, trade_id) {
        if (trade.state == TradeState::Published) | (trade.state == TradeState::Countered) {
            if !trade.whitelisted_users.is_empty() {
                if !trade.whitelisted_users.contains(sender) {
                    Err(ContractError::AddressNotWhitelisted {})
                } else {
                    Ok(trade)
                }
            } else {
                Ok(trade)
            }
        } else {
            Err(ContractError::NotCounterable {})
        }
    } else {
        Err(ContractError::NotFoundInTradeInfo {})
    }
}
