use cosmwasm_std::{
    to_json_binary, Addr, Deps, Env, Order, QueryRequest, StdError, StdResult, WasmQuery,
};
use cw721::{ApprovalResponse, Cw721QueryMsg, Expiration, OwnerOfResponse};
use cw_storage_plus::Bound;

#[cfg(feature = "sg")]
use sg721_base::QueryMsg as Sg721QueryMsg;

use crate::{
    error::ContractError,
    msg::{
        CollateralResponse,
        MultipleCollateralsAllResponse,
        MultipleCollateralsResponse,
        MultipleOffersResponse,
        OfferResponse,
        // QueryFilters,
    },
    state::{
        get_actual_state, get_offer, lender_offers, BorrowerInfo, CollateralInfo, Config,
        BORROWER_INFO, COLLATERAL_INFO, CONFIG,
    },
};

// settings for pagination
const MAX_QUERY_LIMIT: u32 = 150;
const DEFAULT_QUERY_LIMIT: u32 = 10;

pub fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

// confirm ownership
pub fn is_nft_owner(
    deps: Deps,
    sender: Addr,
    nft_address: String,
    token_id: String,
) -> Result<(), ContractError> {
    let owner_response: OwnerOfResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: nft_address,
            msg: to_json_binary(&Cw721QueryMsg::OwnerOf {
                token_id,
                include_expired: None,
            })?,
        }))?;

    if owner_response.owner != sender {
        return Err(ContractError::SenderNotOwner {});
    }
    Ok(())
}

// confirm ownership
#[cfg(feature = "sg")]
pub fn is_sg721_owner(
    deps: Deps,
    sender: Addr,
    nft_address: String,
    token_id: String,
) -> Result<(), ContractError> {
    let owner_response: OwnerOfResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: nft_address,
            msg: to_json_binary(&Sg721QueryMsg::OwnerOf {
                token_id,
                include_expired: None,
            })?,
        }))?;

    if owner_response.owner != sender {
        return Err(ContractError::SenderNotOwner {});
    }
    Ok(())
}

// confirm token approval
pub fn is_approved_cw721(
    deps: Deps,
    env: Env,
    sender: Addr,
    nft_address: String,
    token_id: String,
) -> Result<(), ContractError> {
    let approval_response: ApprovalResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: nft_address,
            msg: to_json_binary(&Cw721QueryMsg::Approval {
                token_id,
                spender: sender.to_string(),
                include_expired: None,
            })?,
        }))?;

    if approval_response.approval.expires <= Expiration::AtHeight(env.block.height) {
        return Err(ContractError::TokenApprovalIssue {});
    }
    Ok(())
}

// confirm token approval
#[cfg(feature = "sg")]
pub fn is_approved_sg721(
    deps: Deps,
    env: Env,
    sender: Addr,
    nft_address: String,
    token_id: String,
) -> Result<(), ContractError> {
    let approval_response: ApprovalResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: nft_address,
            msg: to_json_binary(&Sg721QueryMsg::Approval {
                token_id,
                spender: sender.to_string(),
                include_expired: None,
            })?,
        }))?;

    if approval_response.approval.expires <= Expiration::AtHeight(env.block.height) {
        return Err(ContractError::TokenApprovalIssue {});
    }
    Ok(())
}

pub fn query_borrower_info(deps: Deps, borrower: String) -> StdResult<BorrowerInfo> {
    let borrower = deps.api.addr_validate(&borrower)?;
    BORROWER_INFO
        .load(deps.storage, &borrower)
        .map_err(|_| StdError::generic_err("UnknownBorrower"))
}

// queries a loan given an address and loan id
pub fn query_collateral_info(
    deps: Deps,
    _env: Env,
    borrower: String,
    loan_id: u64,
) -> StdResult<CollateralInfo> {
    let borrower = deps.api.addr_validate(&borrower)?;
    COLLATERAL_INFO.load(deps.storage, (borrower, loan_id))
}

pub fn query_collaterals(
    deps: Deps,
    borrower: String,
    start_after: Option<u64>,
    limit: Option<u32>,
    // filters: Option<QueryFilters>,
) -> StdResult<MultipleCollateralsResponse> {
    let borrower = deps.api.addr_validate(&borrower)?;
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let collaterals: Vec<CollateralResponse> = COLLATERAL_INFO
        .prefix(borrower.clone())
        .range(deps.storage, None, start, Order::Descending)
        .map(|result| {
            result.map(|(loan_id, loan_info)| CollateralResponse {
                borrower: borrower.to_string(),
                loan_id,
                collateral: loan_info.clone(),
                loan_state: loan_info.state,
            })
        })
        .take(limit)
        .collect::<Result<Vec<CollateralResponse>, StdError>>()?;

    Ok(MultipleCollateralsResponse {
        next_collateral: if collaterals.len() == limit {
            collaterals.last().map(|last| last.loan_id)
        } else {
            None
        },
        collaterals,
    })
}

pub fn query_offer_info(deps: Deps, global_offer_id: String) -> StdResult<OfferResponse> {
    let offer_info = get_offer(deps.storage, &global_offer_id)?;

    Ok(OfferResponse {
        global_offer_id,
        offer_info,
    })
}

pub fn query_all_collaterals(
    deps: Deps,
    start_after: Option<(String, u64)>,
    limit: Option<u32>,
    // filters: Option<QueryFilters>,
) -> StdResult<MultipleCollateralsAllResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after
        .map::<Result<Bound<_>, StdError>, _>(|start_after| {
            let borrower = deps.api.addr_validate(&start_after.0)?;
            Ok(Bound::exclusive((borrower, start_after.1)))
        })
        .transpose()?;

    let collaterals: Vec<CollateralResponse> = COLLATERAL_INFO
        .range(deps.storage, None, start, Order::Descending)
        .map(|result| {
            result.map(|(loan_id, loan_info)| CollateralResponse {
                borrower: loan_id.0.to_string(),
                loan_id: loan_id.1,
                collateral: loan_info.clone(),
                loan_state: loan_info.state,
            })
        })
        .take(limit)
        .collect::<Result<Vec<CollateralResponse>, StdError>>()?;

    Ok(MultipleCollateralsAllResponse {
        next_collateral: collaterals
            .last()
            .map(|last| (last.borrower.clone(), last.loan_id)),
        collaterals,
    })
}

pub fn query_offers(
    deps: Deps,
    borrower: String,
    loan_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<MultipleOffersResponse> {
    let borrower = deps.api.addr_validate(&borrower)?;
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let offers: Vec<OfferResponse> = lender_offers()
        .idx
        .loan
        .prefix((borrower, loan_id))
        .range(deps.storage, None, start, Order::Descending)
        .map(|x| match x {
            Ok((key, mut offer_info)) => {
                offer_info.state = get_actual_state(&offer_info, deps.storage)?;
                Ok(OfferResponse {
                    offer_info,
                    global_offer_id: key,
                })
            }
            Err(err) => Err(err),
        })
        .take(limit)
        .collect::<Result<Vec<OfferResponse>, StdError>>()?;

    Ok(MultipleOffersResponse {
        next_offer: offers.last().map(|last| last.global_offer_id.clone()),
        offers,
    })
}

pub fn query_lender_offers(
    deps: Deps,
    lender: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<MultipleOffersResponse> {
    let lender = deps.api.addr_validate(&lender)?;
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let offers: Vec<OfferResponse> = lender_offers()
        .idx
        .lender
        .prefix(lender)
        .range(deps.storage, None, start, Order::Descending)
        .map(|x| {
            x.map(|(key, offer_info)| OfferResponse {
                offer_info,
                global_offer_id: key,
            })
        })
        .take(limit)
        .collect::<StdResult<Vec<OfferResponse>>>()?;

    Ok(MultipleOffersResponse {
        next_offer: offers.last().map(|last| last.global_offer_id.clone()),
        offers,
    })
}

// // used to filter query of collaterals
// pub fn loan_filter(
//     _api: &dyn Api,
//     env: Env,
//     loan_info: &StdResult<CollateralResponse>,
//     filters: &Option<QueryFilters>,
// ) -> bool {
//     if let Some(filters) = filters {
//         let loan = loan_info.as_ref().unwrap();

//         (match &filters.states {
//             Some(state) => {
//                 state.contains(&get_loan_state(env, loan.collateral.clone()).to_string())
//             }
//             None => true,
//         } && match &filters.owner {
//             Some(owner) => loan.borrower == owner.clone(),
//             None => true,
//         })
//     } else {
//         true
//     }
// }
