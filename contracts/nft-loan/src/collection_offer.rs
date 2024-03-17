use cosmwasm_std::{BankMsg, Deps, DepsMut, Env, MessageInfo, Order, StdError};
use utils::{
    state::{is_valid_comment, AssetInfo},
    types::{CosmosMsg, Response},
};

use crate::{
    error::ContractError,
    execute::{_accept_offer_raw, _internal_list_collaterals, _make_offer_raw},
    helpers::assert_listing_fee,
    msg::{CollectionOfferResponse, MultipleCollectionOffersResponse},
    query::{DEFAULT_QUERY_LIMIT, MAX_QUERY_LIMIT},
    state::{LoanTerms, CONFIG},
};

use cosmwasm_std::Addr;
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex};

use crate::state::CollectionOfferInfo;

pub fn execute_make_collection_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: String,
    terms: LoanTerms,
    comment: Option<String>,
) -> Result<Response, ContractError> {
    // We make sure the contract is not locked
    let config = CONFIG.load(deps.storage)?;

    // prevent new listings from being made when contract is frozen
    if config.clone().locks.lock || config.locks.sudo_lock {
        return Err(ContractError::ContractIsLocked {});
    }

    let collection = deps.api.addr_validate(&collection)?;

    // checks comment size
    if !is_valid_comment(&comment.clone().unwrap_or_default()) {
        return Err(ContractError::Std(StdError::generic_err(
            "Comment too long. max = (20000 UTF-8 bytes)",
        )));
    }
    // Make sure the transaction contains funds that match the principle indicated in the terms
    if info.funds.len() != 1 {
        return Err(ContractError::MultipleCoins {});
    } else if terms.principle != info.funds[0].clone() {
        return Err(ContractError::FundsDontMatchTerms {});
    }

    let mut config = CONFIG.load(deps.storage)?;
    config.global_collection_offer_index += 1;
    let global_collection_offer_index = config.global_collection_offer_index;

    collection_offers().save(
        deps.storage,
        &global_collection_offer_index.to_string(),
        &CollectionOfferInfo {
            lender: info.sender.clone(),
            collection: collection.clone(),
            collection_offer_id: global_collection_offer_index,
            terms,
            comment,
        },
    )?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "make_collection_offer")
        .add_attribute("lender", info.sender)
        .add_attribute("collection", collection)
        .add_attribute(
            "collection_offer_id",
            global_collection_offer_index.to_string(),
        ))
}

pub fn execute_withdraw_collection_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection_offer_id: u64,
) -> Result<Response, ContractError> {
    // We load the corresponding collection
    let collection_info =
        collection_offers().load(deps.storage, &collection_offer_id.to_string())?;

    // We make sure the sender is the offerer
    if collection_info.lender != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // We send them the corresponding assets back
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![collection_info.terms.principle],
    });

    // We remove the collection offer
    collection_offers().remove(deps.storage, &collection_offer_id.to_string())?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "remove_collection_offer")
        .add_attribute("lender", info.sender)
        .add_attribute("collection", collection_info.collection))
}

pub fn execute_accept_collection_offer(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection_offer_id: u64,
    token: AssetInfo,
) -> Result<Response, ContractError> {
    // We verify the sender paid the listing fee
    let transfer_fee_msg = assert_listing_fee(deps.as_ref(), info.funds)?;

    // set the borrower
    let borrower = info.sender;

    // We load the corresponding collection
    let collection_info =
        collection_offers().load(deps.storage, &collection_offer_id.to_string())?;

    // We create a collateral listing with the given token
    let (list_attributes, collateral_id) = _internal_list_collaterals(
        deps.branch(),
        env.clone(),
        borrower.clone(),
        vec![token],
        None,
        None,
        None,
    )?;

    // We create an offer on this collateral listing
    // The lender accepts that with a collection offer, they will create an offer and the loan will start as soon as one borrower accepts the terms
    let (global_offer_id, _offer_id) = _make_offer_raw(
        deps.storage,
        env.clone(),
        collection_info.lender.clone(),
        vec![collection_info.terms.principle.clone()],
        borrower.clone(),
        collateral_id,
        collection_info.terms,
        None,
    )?;

    // We accept this collateral listing
    let accept_res = _accept_offer_raw(deps.branch(), env, global_offer_id)?;

    // We remove the collection offer
    collection_offers().remove(deps.storage, &collection_offer_id.to_string())?;

    Ok(Response::new()
        .add_message(transfer_fee_msg)
        .add_attribute("action", "accept_collection_offer")
        .add_attribute("lender", collection_info.lender)
        .add_attribute("collection", collection_info.collection)
        .add_attributes(list_attributes)
        .add_attributes(accept_res.attributes)
        .add_events(accept_res.events)
        .add_submessages(accept_res.messages))
}

pub fn query_collection_offers(
    deps: Deps,
    collection_addr: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<MultipleCollectionOffersResponse, StdError> {
    let collection_addr = deps.api.addr_validate(&collection_addr)?;
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let offers = collection_offers()
        .idx
        .collection
        .prefix(collection_addr)
        .range(deps.storage, None, start, Order::Descending)
        .map(|r| {
            r.map(|(key, collection_offer_info)| CollectionOfferResponse {
                global_offer_id: key,
                collection_offer_info,
            })
        })
        .take(limit)
        .collect::<Result<Vec<CollectionOfferResponse>, _>>()?;

    Ok(MultipleCollectionOffersResponse {
        next_offer: if offers.len() == limit {
            offers.last().map(|last| {
                (
                    last.collection_offer_info.collection.to_string(),
                    last.global_offer_id.clone(),
                )
            })
        } else {
            None
        },
        offers,
    })
}

pub struct CollectionOfferIndexes<'a> {
    pub lender: MultiIndex<'a, Addr, CollectionOfferInfo, String>,
    pub collection: MultiIndex<'a, Addr, CollectionOfferInfo, String>,
}

impl<'a> IndexList<CollectionOfferInfo> for CollectionOfferIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CollectionOfferInfo>> + '_> {
        let v: Vec<&dyn Index<CollectionOfferInfo>> = vec![&self.lender, &self.collection];
        Box::new(v.into_iter())
    }
}

pub fn collection_offers<'a>(
) -> IndexedMap<'a, &'a str, CollectionOfferInfo, CollectionOfferIndexes<'a>> {
    let indexes = CollectionOfferIndexes {
        lender: MultiIndex::new(
            |_, d: &CollectionOfferInfo| d.lender.clone(),
            "collection_offers",
            "collection_offers__lender",
        ),
        collection: MultiIndex::new(
            |_, d: &CollectionOfferInfo| d.collection.clone(),
            "collection_offers",
            "collection_offers__collection",
        ),
    };
    IndexedMap::new("collection_offers", indexes)
}
