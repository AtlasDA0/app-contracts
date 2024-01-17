use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Env, StdError, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use utils::state::AssetInfo;

use crate::error::ContractError;

pub const CONFIG: Item<Config> = Item::new("config");
pub const COLLATERAL_INFO: Map<(Addr, u64), CollateralInfo> = Map::new("collateral_info");
pub const BORROWER_INFO: Map<&Addr, BorrowerInfo> = Map::new("borrower_info");

#[cw_serde]
pub struct OwnerStruct {
    pub owner: Addr,
    pub new_owner: Option<Addr>,
}

#[cw_serde]
pub struct Config {
    /// The name of the smart contract
    pub name: String,
    /// The admin of the smart contract
    pub owner: Addr,
    /// The address which all generated fees are sent to
    pub fee_distributor: Addr,
    /// A % used to calculate how much of a loan interest is
    /// sent to the fee_distributor
    pub fee_rate: Decimal,
    /// Tracks the number of offers made across all loans
    pub global_offer_index: u64,
    /// The expected token denomination being sent when starting a loan workflow
    pub deposit_fee_denom: Vec<String>,
    /// The expected token amount when starting a loan workflow
    pub deposit_fee_amount: u128,
}

#[cw_serde]
pub struct CollateralInfo {
    pub terms: Option<LoanTerms>,
    pub associated_assets: Vec<AssetInfo>,
    pub list_date: Timestamp,
    pub state: LoanState,
    pub offer_amount: u64,
    pub active_offer: Option<String>,
    pub start_block: Option<u64>,
    pub comment: Option<String>,
    pub loan_preview: Option<AssetInfo>, // The preview can only be a CW1155 or a CW721 token.
}

impl Default for CollateralInfo {
    fn default() -> Self {
        Self {
            terms: None,
            associated_assets: vec![],
            list_date: Timestamp::from_nanos(0),
            comment: None,
            state: LoanState::Published,
            offer_amount: 0u64,
            active_offer: None,
            start_block: None,
            loan_preview: None,
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct BorrowerInfo {
    pub last_collateral_id: u64,
}

#[cw_serde]
pub struct OfferInfo {
    pub lender: Addr,
    pub borrower: Addr,
    pub loan_id: u64,
    pub offer_id: u64,
    pub terms: LoanTerms,
    pub state: OfferState,
    pub list_date: Timestamp,
    pub deposited_funds: Option<Coin>,
    pub comment: Option<String>,
}

#[cw_serde]
pub struct LoanTerms {
    pub principle: Coin,
    pub interest: Uint128,
    pub duration_in_blocks: u64,
}

#[cw_serde]
pub enum LoanState {
    Published,
    Started,
    Defaulted,
    Ended,
    Inactive,
}

#[cw_serde]
pub enum OfferState {
    Published,
    Accepted,
    Refused,
    Cancelled,
}

pub struct LenderOfferIndexes<'a> {
    pub lender: MultiIndex<'a, Addr, OfferInfo, String>,
    pub borrower: MultiIndex<'a, Addr, OfferInfo, String>,
    pub loan: MultiIndex<'a, (Addr, u64), OfferInfo, String>,
}

impl<'a> IndexList<OfferInfo> for LenderOfferIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<OfferInfo>> + '_> {
        let v: Vec<&dyn Index<OfferInfo>> = vec![&self.lender, &self.borrower, &self.loan];
        Box::new(v.into_iter())
    }
}

pub fn lender_offers<'a>() -> IndexedMap<'a, &'a str, OfferInfo, LenderOfferIndexes<'a>> {
    let indexes = LenderOfferIndexes {
        lender: MultiIndex::new(
            |_, d: &OfferInfo| d.lender.clone(),
            "lender_offers",
            "lender_offers__lenderr",
        ),
        borrower: MultiIndex::new(
            |_, d: &OfferInfo| d.borrower.clone(),
            "lender_offers",
            "lender_offers__borrower",
        ),
        loan: MultiIndex::new(
            |_, d: &OfferInfo| (d.borrower.clone(), d.loan_id),
            "lender_offers",
            "lender_offers__collateral",
        ),
    };
    IndexedMap::new("lender_offers", indexes)
}

pub fn is_loan_modifiable(collateral: &CollateralInfo) -> Result<(), ContractError> {
    match collateral.state {
        LoanState::Published => Ok(()),
        _ => return Err(ContractError::NotModifiable {}),
    }
}
pub fn is_loan_acceptable(collateral: &CollateralInfo) -> Result<(), ContractError> {
    match collateral.state {
        LoanState::Published => Ok(()),
        _ => return Err(ContractError::NotAcceptable {}),
    }
}

pub fn is_loan_counterable(collateral: &CollateralInfo) -> Result<(), ContractError> {
    match collateral.state {
        LoanState::Published => Ok(()),
        _ => return Err(ContractError::NotCounterable {}),
    }
}

pub fn is_offer_refusable(
    collateral: &CollateralInfo,
    offer_info: &OfferInfo,
) -> Result<(), ContractError> {
    is_loan_counterable(collateral).map_err(|_| ContractError::NotRefusable {})?;
    match offer_info.state {
        OfferState::Published => Ok(()),
        _ => return Err(ContractError::NotRefusable {}),
    }
}

pub fn can_repay_loan(
    storage: &dyn Storage,
    env: Env,
    collateral: &CollateralInfo,
) -> Result<(), ContractError> {
    if is_loan_defaulted(storage, env, collateral).is_ok() {
        return Err(ContractError::WrongLoanState {
            state: LoanState::Defaulted {},
        });
    } else if collateral.state != LoanState::Started {
        return Err(ContractError::WrongLoanState {
            state: collateral.state.clone(),
        });
    } else {
        Ok(())
    }
}

pub fn is_loan_defaulted(
    storage: &dyn Storage,
    env: Env,
    collateral: &CollateralInfo,
) -> Result<(), ContractError> {
    // If there is no offer, the loan can't be defaulted
    let offer: OfferInfo = get_active_loan(storage, collateral)?;
    match &collateral.state {
        LoanState::Started => {
            if collateral.start_block.unwrap() + offer.terms.duration_in_blocks < env.block.height {
                Ok(())
            } else {
                return Err(ContractError::WrongLoanState {
                    state: LoanState::Started,
                });
            }
        }
        LoanState::Defaulted => Ok(()),
        _ => {
            return Err(ContractError::WrongLoanState {
                state: collateral.state.clone(),
            })
        }
    }
}

pub fn get_active_loan(
    storage: &dyn Storage,
    collateral: &CollateralInfo,
) -> Result<OfferInfo, ContractError> {
    let global_offer_id = collateral
        .active_offer
        .as_ref()
        .ok_or(ContractError::OfferNotFound {})?;
    Ok(get_offer(storage, global_offer_id)?)
}

pub fn is_lender(
    storage: &dyn Storage,
    lender: Addr,
    global_offer_id: &str,
) -> Result<OfferInfo, ContractError> {
    let offer = get_offer(storage, global_offer_id)?;
    if lender != offer.lender {
        return Err(ContractError::Unauthorized {});
    }
    Ok(offer)
}

pub fn is_collateral_withdrawable(collateral: &CollateralInfo) -> Result<(), ContractError> {
    match collateral.state {
        LoanState::Published => Ok(()),
        _ => return Err(ContractError::NotWithdrawable {}),
    }
}

pub fn is_offer_borrower(
    storage: &dyn Storage,
    borrower: Addr,
    global_offer_id: &str,
) -> Result<OfferInfo, ContractError> {
    let offer = get_offer(storage, global_offer_id)?;
    if borrower != offer.borrower {
        return Err(ContractError::Unauthorized {});
    }
    Ok(offer)
}

pub fn is_active_lender(
    storage: &dyn Storage,
    lender: Addr,
    collateral: &CollateralInfo,
) -> Result<OfferInfo, ContractError> {
    let offer = get_active_loan(storage, collateral)?;
    if lender != offer.lender {
        return Err(ContractError::Unauthorized {});
    }
    Ok(offer)
}

pub fn save_offer(
    storage: &mut dyn Storage,
    global_offer_id: &str,
    offer_info: OfferInfo,
) -> StdResult<()> {
    lender_offers().save(storage, global_offer_id, &offer_info)
}

pub fn get_offer(storage: &dyn Storage, global_offer_id: &str) -> StdResult<OfferInfo> {
    let mut offer_info = lender_offers()
        .load(storage, global_offer_id)
        .map_err(|_| StdError::generic_err("invalid offer"))?;

    offer_info.state = get_actual_state(&offer_info, storage)?;

    Ok(offer_info)
}

pub fn get_actual_state(offer_info: &OfferInfo, storage: &dyn Storage) -> StdResult<OfferState> {
    let collateral_info =
        COLLATERAL_INFO.load(storage, (offer_info.borrower.clone(), offer_info.loan_id))?;

    // We check the status of the offer.
    // A refused offer isn't marked as such but depends on the overlying collateral info state
    Ok(match &offer_info.state {
        OfferState::Published => {
            if collateral_info.state != LoanState::Published {
                OfferState::Refused
            } else {
                OfferState::Published
            }
        }
        _ => offer_info.state.clone(),
    })
}
