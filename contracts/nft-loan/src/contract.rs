#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    coin, ensure, ensure_eq, entry_point, to_json_binary, Binary, Coin, Decimal, Deps, DepsMut,
    Empty, Env, MessageInfo, StdResult,
};

use cw2::set_contract_version;

use utils::{
    state::{is_valid_name, Locks, SudoMsg, NATIVE_DENOM},
    types::Response,
};

use crate::{
    collection_offer::execute_accept_collection_offer,
    query::{
        query_all_collaterals, query_borrower_info, query_collateral_info, query_collaterals,
        query_config, query_lender_offers, query_offer_info, query_offers,
    },
};
use crate::{
    collection_offer::execute_make_collection_offer,
    execute::{
        accept_loan, accept_offer, cancel_offer, execute_toggle_lock, list_collaterals, make_offer,
        modify_collaterals, refuse_offer, repay_borrowed_funds, withdraw_collateral,
        withdraw_defaulted_loan, withdraw_refused_offer,
    },
};
use crate::{
    collection_offer::execute_withdraw_collection_offer,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};
use crate::{
    collection_offer::query_collection_offers,
    state::{Config, CONFIG, STATIC_LOAN_LISTING_FEE},
};
use crate::{error::ContractError, execute::execute_sudo_toggle_lock};
// version info for migration info
const CONTRACT_NAME: &str = concat!("crates.io:", env!("CARGO_CRATE_NAME"));
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // valid interest fee
    ensure!(
        msg.fee_rate > Decimal::zero() && msg.fee_rate <= Decimal::one(),
        ContractError::InvalidFeeRate {}
    );
    // valid name
    if !is_valid_name(&msg.name) {
        return Err(ContractError::InvalidName {});
    }

    // define the accepted fee coins
    let listing_fee = match msg.listing_fee_coins {
        Some(cc_msg) => cc_msg,
        None => vec![coin(STATIC_LOAN_LISTING_FEE, NATIVE_DENOM)],
    };

    let data = Config {
        name: msg.name,
        owner: deps
            .api
            .addr_validate(&msg.owner.unwrap_or_else(|| info.sender.to_string()))?,
        treasury_addr: deps.api.addr_validate(&msg.treasury_addr)?,
        fee_rate: msg.fee_rate,
        global_offer_index: 0,
        global_collection_offer_index: 0,
        listing_fee_coins: listing_fee,
        locks: Locks {
            lock: false,
            sudo_lock: false,
        },
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(deps.storage, &data)?;
    Ok(Response::new()
        .add_attribute("action", "initialization")
        .add_attribute("contract", "sg_nft_loans")
        .add_attribute("owner", info.sender))
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ListCollaterals {
            tokens,
            terms,
            comment,
            loan_preview,
        } => Ok(list_collaterals(
            deps,
            env,
            info,
            tokens,
            terms,
            comment,
            loan_preview,
        )?),
        ExecuteMsg::ModifyCollaterals {
            loan_id,
            terms,
            comment,
            loan_preview,
        } => modify_collaterals(deps, env, info, loan_id, terms, comment, loan_preview),
        ExecuteMsg::WithdrawCollaterals { loan_id } => {
            withdraw_collateral(deps, env, info, loan_id)
        }
        ExecuteMsg::AcceptLoan {
            borrower,
            loan_id,
            comment,
        } => accept_loan(deps, env, info, borrower, loan_id, comment),
        ExecuteMsg::AcceptOffer { global_offer_id } => {
            accept_offer(deps, env, info, global_offer_id)
        }
        ExecuteMsg::MakeOffer {
            borrower,
            loan_id,
            terms,
            comment,
        } => make_offer(deps, env, info, borrower, loan_id, terms, comment),
        ExecuteMsg::CancelOffer { global_offer_id } => {
            cancel_offer(deps, env, info, global_offer_id)
        }
        ExecuteMsg::RefuseOffer { global_offer_id } => {
            refuse_offer(deps, env, info, global_offer_id)
        }
        ExecuteMsg::WithdrawRefusedOffer { global_offer_id } => {
            withdraw_refused_offer(deps, env, info, global_offer_id)
        }
        ExecuteMsg::RepayBorrowedFunds { loan_id } => {
            repay_borrowed_funds(deps, env, info, loan_id)
        }
        ExecuteMsg::WithdrawDefaultedLoan { borrower, loan_id } => {
            withdraw_defaulted_loan(deps, env, info, borrower, loan_id)
        }
        ExecuteMsg::SetOwner { owner } => set_owner(deps, env, info, owner),
        ExecuteMsg::SetFeeDestination { treasury_addr } => {
            set_fee_distributor(deps, env, info, treasury_addr)
        }
        ExecuteMsg::SetListingCoins { listing_fee_coins } => {
            set_listing_coins(deps, env, info, listing_fee_coins)
        }
        ExecuteMsg::SetFeeRate { fee_rate } => set_fee_rate(deps, env, info, fee_rate),
        ExecuteMsg::ToggleLock { lock } => execute_toggle_lock(deps, info, env, lock),
        ExecuteMsg::MakeCollectionOffer {
            collection,
            terms,
            comment,
        } => execute_make_collection_offer(deps, env, info, collection, terms, comment),
        ExecuteMsg::WithdrawCollectionOffer {
            collection_offer_id,
        } => execute_withdraw_collection_offer(deps, env, info, collection_offer_id),
        ExecuteMsg::AcceptCollectionOffer {
            token,
            collection_offer_id,
        } => execute_accept_collection_offer(deps, env, info, collection_offer_id, token),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::BorrowerInfo { borrower } => {
            to_json_binary(&query_borrower_info(deps, borrower)?)
        }
        QueryMsg::CollateralInfo { borrower, loan_id } => {
            to_json_binary(&query_collateral_info(deps, env, borrower, loan_id)?)
        }
        QueryMsg::Collaterals {
            borrower,
            start_after,
            limit,
        } => to_json_binary(&query_collaterals(deps, borrower, start_after, limit)?),
        QueryMsg::AllCollaterals { start_after, limit } => {
            to_json_binary(&query_all_collaterals(deps, start_after, limit)?)
        }
        QueryMsg::OfferInfo { global_offer_id } => {
            to_json_binary(&query_offer_info(deps, global_offer_id)?)
        }
        QueryMsg::Offers {
            borrower,
            loan_id,
            start_after,
            limit,
        } => to_json_binary(&query_offers(deps, borrower, loan_id, start_after, limit)?),
        QueryMsg::LenderOffers {
            lender,
            start_after,
            limit,
        } => to_json_binary(&query_lender_offers(deps, lender, start_after, limit)?),
        QueryMsg::CollectionOffers {
            collection,
            start_after,
            limit,
        } => to_json_binary(&query_collection_offers(
            deps,
            collection,
            start_after,
            limit,
        )?),
    }
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

// sets a new owner of contract
pub fn set_owner(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    ensure_eq!(info.sender, config.owner, ContractError::Unauthorized {});
    let new_admin = deps.api.addr_validate(&new_owner)?;
    config.owner = new_admin;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default()
        .add_attribute("action", "new owner")
        .add_attribute("new owner", new_owner))
}

/// Owner only functions
/// Sets a new fee-distributor contract
pub fn set_fee_distributor(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    treasury_addr: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    ensure_eq!(info.sender, config.owner, ContractError::Unauthorized {});
    config.treasury_addr = deps.api.addr_validate(&treasury_addr)?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default()
        .add_attribute("action", "changed-contract-parameter")
        .add_attribute("parameter", "fee_distributor")
        .add_attribute("value", treasury_addr))
}

/// Sets a new fee rate
/// fee_rate is in units of a 1/100_000th, so e.g. if fee_rate=5_000, the fee_rate is 5%
/// It correspond to the part of interests that are kept by the organisation (for redistribution and DAO purposes)
pub fn set_fee_rate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_fee_rate: Decimal,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    ensure_eq!(info.sender, config.owner, ContractError::Unauthorized {});
    // Check the fee distribution
    if new_fee_rate >= Decimal::one() {
        return Err(ContractError::NotAcceptable {});
    }
    config.fee_rate = new_fee_rate;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "changed-contract-parameter")
        .add_attribute("parameter", "fee_rate")
        .add_attribute("value", new_fee_rate.to_string()))
}

pub fn set_listing_coins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    listing_fee_coins: Vec<Coin>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    ensure_eq!(info.sender, config.owner, ContractError::Unauthorized {});

    config.listing_fee_coins = listing_fee_coins;

    CONFIG.save(deps.storage, &config)?;
    Ok(
        Response::default()
            .add_attribute("action", "changed-contract-parameter")
            .add_attribute("parameter", "fee_distributor"),
        // TODO: add new value in response for bookeeping updates
        // .add_attribute("value", listing_fee_coins)
    )
}
