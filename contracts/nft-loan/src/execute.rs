#[cfg(not(feature = "vanilla"))]
use {
    crate::query::{is_approved_sg721, is_sg721_owner},
    sg721::ExecuteMsg as Sg721ExecuteMsg,
    utils::state::Sg721Token,
};

use {
    crate::{
        error::{self, ContractError},
        query::{is_approved_cw721, is_nft_owner},
        state::{
            can_repay_loan, get_active_loan, get_offer, is_active_lender,
            is_collateral_withdrawable, is_lender, is_loan_acceptable, is_loan_counterable,
            is_loan_defaulted, is_loan_modifiable, is_offer_borrower, is_offer_refusable,
            lender_offers, save_offer, BorrowerInfo, CollateralInfo, LoanState, LoanTerms,
            OfferInfo, OfferState, BORROWER_INFO, COLLATERAL_INFO, CONFIG,
        },
    },
    cosmwasm_std::{
        coins, ensure_eq, Addr, BankMsg, Coin, Decimal, DepsMut, Empty, Env, MessageInfo, StdError,
        StdResult, Storage,
    },
    cw721::Cw721ExecuteMsg,
    cw721_base::Extension,
    utils::{
        state::{into_cosmos_msg, is_valid_comment, AssetInfo, Cw721Coin},
        types::{CosmosMsg, Response},
    },
};

/// Signals the listing of multiple collaterals in the same loan.
/// This is the first entry point of the loan flow.
/// Users signal they want a loan against their collaterals for other users to accept their terms,
/// in exchange of interest paid at the end of the loan duration
/// Their collateral is not deposited at this stage as this system is non-custodial.
/// Users lock their assets only when the deal is made (`accept_loan` or `accept_offer` functions)
/// The borrower (the person that deposits collaterals) can specify terms at which they wish to borrow funds against their collaterals.
/// If terms are specified, fund lenders can accept the loan directly.
/// If not, lenders can propose terms than may be accepted by the borrower in return to start the loan
/// This deposit function allows CW721 and SG721 tokens to be listed
pub fn list_collaterals(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tokens: Vec<AssetInfo>,
    terms: Option<LoanTerms>,
    comment: Option<String>,
    loan_preview: Option<AssetInfo>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // prevent new listings from being made when contract is frozen
    if config.clone().locks.lock || config.locks.sudo_lock {
        return Err(ContractError::ContractIsLocked {});
    }

    // set the borrower
    let borrower = info.sender;

    // ensure atleas one asset has been provided
    if tokens.is_empty() {
        return Err(ContractError::NoAssets {});
    }

    tokens.iter().try_for_each(|token| match token {
        AssetInfo::Cw721Coin(Cw721Coin { address, token_id }) => {
            // asserts borrower is owner of collateral
            is_nft_owner(
                deps.as_ref(),
                borrower.clone(),
                address.to_string(),
                token_id.to_string(),
            )?;
            // asserts nft has been approved for use by loan contract
            is_approved_cw721(
                deps.as_ref(),
                env.clone(),
                borrower.clone(),
                address.clone(),
                token_id.clone(),
            )
        }
        #[cfg(not(feature = "vanilla"))]
        AssetInfo::Sg721Token(Sg721Token { address, token_id }) => {
            // asserts borrower is owner of collateral
            is_sg721_owner(
                deps.as_ref(),
                borrower.clone(),
                address.to_string(),
                token_id.to_string(),
            )?;
            // asserts nft has been approved for use by loan contract
            is_approved_sg721(
                deps.as_ref(),
                env.clone(),
                borrower.clone(),
                address.clone(),
                token_id.clone(),
            )
        }
        _ => Err(ContractError::SenderNotOwner {}),
    })?;

    let fee = info
        .funds
        .iter()
        .find(|c| config.listing_fee_coins.contains(&c))
        .map(|c| Coin::from(c.clone()))
        .unwrap_or_default();

    if !config.listing_fee_coins.contains(&fee) {
        return Err(ContractError::DepositFeeError {});
    }
    // transfer fee to treasury_addr
    let transfer_fee: CosmosMsg = BankMsg::Send {
        to_address: config.treasury_addr.to_string(),
        amount: vec![fee], // only the fee required
    }
    .into();

    // We save the collateral info in our internal structure
    // First we update the number of collateral a user has deposited (to make sure the id assigned is unique)
    let loan_id = BORROWER_INFO
        .update::<_, error::ContractError>(deps.storage, &borrower, |x| match x {
            Some(mut info) => {
                info.last_collateral_id += 1;
                Ok(info)
            }
            None => Ok(BorrowerInfo::default()),
        })?
        .last_collateral_id;

    // Then we verify we can set the asset as preview
    if let Some(preview) = loan_preview.clone() {
        if !tokens.iter().any(|r| *r == preview) {
            return Err(ContractError::AssetNotInLoan {});
        }
    }

    // checks comment size
    if !is_valid_comment(&comment.clone().unwrap_or_default()) {
        return Err(ContractError::Std(StdError::generic_err(
            "Comment too long. max = (20000 UTF-8 bytes)",
        )));
    }

    // Finally we save a collateral info object
    COLLATERAL_INFO.save(
        deps.storage,
        (borrower.clone(), loan_id),
        &CollateralInfo {
            terms,
            associated_assets: tokens,
            list_date: env.block.time,
            comment,
            loan_preview,
            ..Default::default()
        },
    )?;

    // response attributes
    Ok(Response::new()
        .add_message(transfer_fee)
        .add_attribute("action", "deposit_collateral")
        .add_attribute("borrower", borrower)
        .add_attribute("loan_id", loan_id.to_string()))
}

// modify a listing, if possible
pub fn modify_collaterals(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    loan_id: u64,
    terms: Option<LoanTerms>,
    comment: Option<String>,
    loan_preview: Option<AssetInfo>,
) -> Result<Response, ContractError> {
    let borrower = info.sender;

    COLLATERAL_INFO.update(
        deps.storage,
        (borrower.clone(), loan_id),
        |collateral| match collateral {
            // will panic if msg sender is not calling a loan_id it owns
            None => return Err(ContractError::LoanNotFound {}),
            Some(mut collateral) => {
                is_loan_modifiable(&collateral)?;

                if terms.is_some() {
                    collateral.terms = terms;
                }
                if comment.is_some() {
                    if !is_valid_comment(&comment.clone().unwrap_or_default()) {
                        return Err(ContractError::Std(StdError::generic_err(
                            "Comment too long. max = (20000 UTF-8 bytes)",
                        )));
                    }
                    collateral.comment = comment;
                }
                // Then we verify we can set the asset as preview
                if let Some(preview) = loan_preview.clone() {
                    if !collateral.associated_assets.iter().any(|r| *r == preview) {
                        return Err(ContractError::AssetNotInLoan {});
                    }
                    collateral.loan_preview = loan_preview;
                }
                collateral.list_date = env.block.time;

                Ok(collateral)
            }
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "modify_collaterals")
        .add_attribute("borrower", borrower)
        .add_attribute("loan_id", loan_id.to_string()))
}

/// Withdraw an loan collateral listing (cancel a loan collateral)
/// This simply cancels the potential loan.
/// The collateral is not given back as there is not deposited collateral when creating a new loan
/// TODO: remove approval from all collaterals
pub fn withdraw_collateral(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    loan_id: u64,
) -> Result<Response, ContractError> {
    // We query the loan info
    let borrower = info.sender.clone();
    let mut collateral = COLLATERAL_INFO.load(deps.storage, (borrower.clone(), loan_id))?;
    is_collateral_withdrawable(&collateral)?;

    // We update the internal state, the loan proposal is no longer valid
    collateral.state = LoanState::Inactive;
    COLLATERAL_INFO.save(deps.storage, (borrower.clone(), loan_id), &collateral)?;

    Ok(Response::new()
        // .add_message(return_fee)
        .add_attribute("action", "withdraw_collateral")
        .add_attribute("event", "cancel_loan")
        .add_attribute("borrower", borrower)
        .add_attribute("loan_id", loan_id.to_string()))
}

/// Accept a loan and its terms directly
/// As soon as the lender executes this messages, the loan starts and the borrower will need to repay the loan before the term
pub fn accept_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    borrower: String,
    loan_id: u64,
    comment: Option<String>,
) -> Result<Response, ContractError> {
    // We query the loan info
    let borrower_addr = deps.api.addr_validate(&borrower)?;
    let collateral = COLLATERAL_INFO.load(deps.storage, (borrower_addr.clone(), loan_id))?;

    // limits comment size
    if !is_valid_comment(&comment.clone().unwrap_or_default()) {
        return Err(ContractError::Std(StdError::generic_err(
            "Comment too long. max = (20000 UTF-8 bytes)",
        )));
    }

    // We start by making an offer with exactly the same terms as the depositor specified
    let terms: LoanTerms = collateral.terms.ok_or(ContractError::NoTermsSpecified {})?;
    let (global_offer_id, _offer_id) = _make_offer_raw(
        deps.storage,
        env.clone(),
        info,
        borrower_addr,
        loan_id,
        terms,
        comment,
    )?;

    // Then we make the borrower accept the loan
    let res = _accept_offer_raw(deps, env, global_offer_id)?;

    Ok(res.add_attribute("action_type", "accept_loan"))
}

// It verifies an offer can be made for the current loan
// It verifies the sent funds match the principle indicated in the terms
// And then saves the new offer in the internal storage
fn _make_offer_raw(
    storage: &mut dyn Storage,
    env: Env,
    info: MessageInfo,
    borrower: Addr,
    loan_id: u64,
    terms: LoanTerms,
    comment: Option<String>,
) -> Result<(String, u64), ContractError> {
    let mut contract_config = CONFIG.load(storage)?;

    // prevents loan from being accepted or made if contract is locked
    if contract_config.clone().locks.lock || contract_config.locks.sudo_lock {
        return Err(ContractError::ContractIsLocked {});
    }

    let mut collateral: CollateralInfo =
        COLLATERAL_INFO.load(storage, (borrower.clone(), loan_id))?;
    is_loan_counterable(&collateral)?;

    // Make sure the transaction contains funds that match the principle indicated in the terms
    if info.funds.len() != 1 {
        return Err(ContractError::MultipleCoins {});
    } else if terms.principle != info.funds[0].clone() {
        return Err(ContractError::FundsDontMatchTerms {});
    }

    // We add the new offer to the collateral object
    collateral.offer_amount += 1;
    COLLATERAL_INFO.save(storage, (borrower.clone(), loan_id), &collateral)?;
    let offer_id = collateral.offer_amount;

    // We save this new offer
    contract_config.global_offer_index += 1;
    let global_offers = lender_offers();
    global_offers.save(
        storage,
        &contract_config.global_offer_index.to_string(),
        &OfferInfo {
            lender: info.sender,
            borrower,
            loan_id,
            offer_id,
            terms: terms.clone(),
            state: OfferState::Published,
            list_date: env.block.time,
            deposited_funds: Some(terms.principle),
            comment,
        },
    )?;

    CONFIG.save(storage, &contract_config)?;

    Ok((contract_config.global_offer_index.to_string(), offer_id))
}

// internal function that begins actually starts the loanl if possible
fn _accept_offer_raw(
    deps: DepsMut,
    env: Env,
    global_offer_id: String,
) -> Result<Response, ContractError> {
    let mut offer = get_offer(deps.storage, &global_offer_id)?;

    let borrower = offer.borrower.clone();
    let loan_id = offer.loan_id;
    let mut collateral = COLLATERAL_INFO.load(deps.storage, (borrower.clone(), loan_id))?;
    is_loan_acceptable(&collateral)?;

    // We verify the offer is still valid
    if offer.state == OfferState::Published {
        // We can start the loan now !
        collateral.state = LoanState::Started;
        collateral.start_block = Some(env.block.height);
        collateral.active_offer = Some(global_offer_id.clone());
        offer.state = OfferState::Accepted;

        COLLATERAL_INFO.save(deps.storage, (borrower.clone(), loan_id), &collateral)?;
        save_offer(deps.storage, &global_offer_id, offer.clone())?;
    } else {
        return Err(ContractError::WrongOfferState { state: offer.state });
    };

    // We transfer the funds directly when the offer is accepted
    let fund_messages = _withdraw_offer_unsafe(borrower.clone(), &offer)?;

    // We transfer the nfts directly from the owner's wallets when the offer is accepted
    let asset_messages: Vec<CosmosMsg> = collateral
        .associated_assets
        .iter()
        .map(|token| match token {
            AssetInfo::Cw721Coin(Cw721Coin { address, token_id }) => {
                // (Audit results)
                // Before transferring the NFT, we make sure the current NFT owner is indeed the borrower of funds
                // Otherwise, this would cause anyone to be able to create loans in the name of the owner if a bad approval was done
                is_nft_owner(
                    deps.as_ref(),
                    borrower.clone(),
                    address.to_string(),
                    token_id.to_string(),
                )?;

                Ok(into_cosmos_msg(
                    Cw721ExecuteMsg::TransferNft {
                        recipient: env.contract.address.clone().into(),
                        token_id: token_id.to_string(),
                    },
                    address,
                    None,
                )?)
            }
            #[cfg(not(feature = "vanilla"))]
            AssetInfo::Sg721Token(Sg721Token { address, token_id }) => {
                is_sg721_owner(
                    deps.as_ref(),
                    borrower.clone(),
                    address.to_string(),
                    token_id.to_string(),
                )?;

                Ok(into_cosmos_msg(
                    Sg721ExecuteMsg::<Extension, Empty>::TransferNft {
                        recipient: env.contract.address.clone().into(),
                        token_id: token_id.to_string(),
                    },
                    address,
                    None,
                )?)
            }
            _ => Err(ContractError::WrongAssetDeposited {}),
        })
        .collect::<Result<Vec<CosmosMsg>, ContractError>>()?;

    Ok(Response::new()
        .add_message(fund_messages)
        .add_messages(asset_messages)
        .add_attribute("action", "start_loan")
        .add_attribute("denom_borrowed", offer.terms.principle.denom)
        .add_attribute("amount_borrowed", offer.terms.principle.amount.to_string())
        .add_attribute("borrower", borrower)
        .add_attribute("lender", offer.lender)
        .add_attribute("loan_id", loan_id.to_string())
        .add_attribute("global_offer_id", global_offer_id))
}

/// This creates withdraw messages to withdraw the funds from an offer (to the lender of the borrower depending on the situation
/// This function does not do any checks on the validity of the procedure
/// Be careful when using this internal function
pub fn _withdraw_offer_unsafe(
    recipient: Addr,
    offer_info: &OfferInfo,
) -> Result<BankMsg, ContractError> {
    // We get the funds to withdraw
    let funds_to_withdraw = offer_info
        .deposited_funds
        .clone()
        .ok_or(ContractError::NoFundsToWithdraw {})?;

    Ok(BankMsg::Send {
        to_address: recipient.to_string(),
        amount: vec![funds_to_withdraw],
    })
}

/// Accept an offer someone made for your collateral
/// As soon as the borrower executes this messages, the loan starts and the they will need to repay the loan before the term
pub fn accept_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    global_offer_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // prevent offers from being accepted when contract is frozen
    if config.clone().locks.lock || config.locks.sudo_lock {
        return Err(ContractError::ContractIsLocked {});
    }

    // We make sure the caller is the borrower
    is_offer_borrower(deps.storage, info.sender, &global_offer_id)?;

    // We accept the offer
    let res = _accept_offer_raw(deps, env, global_offer_id)?;

    Ok(res.add_attribute("action_type", "accept_offer"))
}

/// Make an offer (offer some terms) to lend some money against someone's collateral
/// The borrower will then be able to accept those terms if they please them
pub fn make_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    borrower: String,
    loan_id: u64,
    terms: LoanTerms,
    comment: Option<String>,
) -> Result<Response, ContractError> {
    // We query the loan info

    let borrower = deps.api.addr_validate(&borrower)?;

    // checks comment size
    if !is_valid_comment(&comment.clone().unwrap_or_default()) {
        return Err(ContractError::Std(StdError::generic_err(
            "Comment too long. max = (20000 UTF-8 bytes)",
        )));
    }
    let (global_offer_id, _offer_id) = _make_offer_raw(
        deps.storage,
        env,
        info.clone(),
        borrower.clone(),
        loan_id,
        terms,
        comment,
    )?;

    Ok(Response::new()
        .add_attribute("action", "make_offer")
        .add_attribute("borrower", borrower)
        .add_attribute("lender", info.sender)
        .add_attribute("loan_id", loan_id.to_string())
        .add_attribute("global_offer_id", global_offer_id))
}

/// Cancel an offer you made in case the market changes or whatever
/// The borrower won't be able to accept the loan if you cancel it
/// You get the assets you offered back when calling this message
pub fn cancel_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    global_offer_id: String,
) -> Result<Response, ContractError> {
    let lender = info.sender;
    // We need to verify the offer exists and it belongs to the address calling the contract and that's in the right state to be cancelled
    let mut offer_info = is_lender(deps.storage, lender.clone(), &global_offer_id)?;
    if offer_info.state != OfferState::Published {
        return Err(ContractError::CantChangeOfferState {
            from: offer_info.state,
            to: OfferState::Cancelled,
        });
    }

    // We query the loan info
    let borrower = offer_info.borrower.clone();
    let loan_id = offer_info.loan_id;
    let collateral = COLLATERAL_INFO.load(deps.storage, (borrower.clone(), loan_id))?;
    // We can cancel an offer only if the Borrower is still searching for a loan (the loan is modifyable)
    is_loan_modifiable(&collateral)?;

    // The funds deposited for lending are withdrawn
    let withdraw_response = _withdraw_offer_unsafe(lender.clone(), &offer_info)?;

    offer_info.state = OfferState::Cancelled;
    offer_info.deposited_funds = None;
    save_offer(deps.storage, &global_offer_id, offer_info)?;

    Ok(Response::new()
        .add_message(withdraw_response)
        .add_attribute("action", "cancel_offer")
        .add_attribute("action", "withdraw_funds")
        .add_attribute("borrower", borrower)
        .add_attribute("lender", lender)
        .add_attribute("loan_id", loan_id.to_string())
        .add_attribute("global_offer_id", global_offer_id))
}

/// Refuse an offer to a borrowers collateral
/// This is needed only for printing and db procedure, and not actually needed in the flow.
/// This however blocks other interactions with the offer (except withdrawing the funds).
/// (Audit results)
/// We need to make sure the owner can only refuse an offer, when :
/// 1. They are still accepting offer (LoanState::Published)
/// 2. The offer is still published
pub fn refuse_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    global_offer_id: String,
) -> Result<Response, ContractError> {
    // We query the loan info
    let borrower = info.sender;

    // We load the offer and collateral info
    let mut offer_info = is_offer_borrower(deps.storage, borrower.clone(), &global_offer_id)?;
    let collateral = COLLATERAL_INFO.load(
        deps.storage,
        (offer_info.clone().borrower, offer_info.loan_id),
    )?;

    // Check the owner can indeed refuse the offer
    is_offer_refusable(&collateral, &offer_info)?;

    // Mark the offer as refused
    offer_info.state = OfferState::Refused;
    save_offer(deps.storage, &global_offer_id, offer_info.clone())?;

    Ok(Response::new()
        .add_attribute("action", "refuse_offer")
        .add_attribute("borrower", borrower)
        .add_attribute("loan_id", offer_info.loan_id.to_string())
        .add_attribute("lender", offer_info.lender)
        .add_attribute("global_offer_id", global_offer_id))
}

/// Withdraw the funds from a refused offer
/// In case the borrower refuses your offer, you need to manually withdraw your funds
pub fn withdraw_refused_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    global_offer_id: String,
) -> Result<Response, ContractError> {
    let lender = info.sender;

    // We need to verify the offer exists and the sender is actually the owner of the offer
    let mut offer_info = is_lender(deps.storage, lender.clone(), &global_offer_id)?;

    if offer_info.state != OfferState::Refused {
        return Err(ContractError::NotWithdrawable {});
    }

    // The funds deposited for lending are withdrawn
    let withdraw_message = _withdraw_offer_unsafe(lender.clone(), &offer_info)?;

    offer_info.deposited_funds = None;
    save_offer(deps.storage, &global_offer_id, offer_info.clone())?;

    Ok(Response::new()
        .add_message(withdraw_message)
        .add_attribute("action", "withdraw_funds")
        .add_attribute("event", "refused_offer")
        .add_attribute("borrower", offer_info.borrower)
        .add_attribute("lender", lender)
        .add_attribute("loan_id", offer_info.loan_id.to_string())
        .add_attribute("global_offer_id", global_offer_id))
}

/// Repay Borrowed funds and get back your collateral
/// This function receives principle + interest funds to end the loan and unlock the collateral
/// This effectively puts an end to the loan.
/// Loans can only be repaid before the period ends.
/// There is not takebacks, no failesafe
pub fn repay_borrowed_funds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    loan_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // We query the loan info
    let borrower = info.sender;
    let mut collateral = COLLATERAL_INFO.load(deps.storage, (borrower.clone(), loan_id))?;
    can_repay_loan(deps.storage, env.clone(), &collateral)?;
    let offer_info = get_active_loan(deps.storage, &collateral)?;

    // We verify the sent funds correspond to the principle + interests
    let interests = offer_info.terms.interest;
    if info.funds.len() != 1 {
        return Err(ContractError::MultipleCoins {});
    } else if offer_info.terms.principle.denom != info.funds[0].denom.clone() {
        return Err(ContractError::FundsDontMatchTerms {});
    } else if offer_info.terms.principle.amount + interests > info.funds[0].amount {
        return Err(ContractError::FundsDontMatchTermsAndPrinciple(
            offer_info.terms.principle.amount + interests,
            info.funds[0].amount,
        ));
    }

    // We save the collateral state
    collateral.state = LoanState::Ended;
    COLLATERAL_INFO.save(deps.storage, (borrower.clone(), loan_id), &collateral)?;

    // We prepare the funds to send back to the lender
    // % of interest expected back
    let lender_payback = interests * (Decimal::one() - config.fee_rate);
    let treasury_payback = info.funds[0].amount - lender_payback;

    let mut res = Response::new();
    // lender is paid back
    if lender_payback.u128() > 0u128 {
        res = res.add_message(BankMsg::Send {
            to_address: offer_info.lender.to_string(),
            amount: coins(lender_payback.u128(), info.funds[0].denom.clone()),
        })
    }

    // add the msg to withdraw_loan collateral back to the borrower*
    res = res.add_messages(_withdraw_loan(
        collateral,
        env.contract.address,
        borrower.clone(),
    )?);

    // And we pay the fee to the treasury
    if treasury_payback.u128() > 0u128 {
        res = res.add_message(BankMsg::Send {
            to_address: config.treasury_addr.to_string(),
            amount: coins(treasury_payback.u128(), info.funds[0].denom.clone()),
        });
    }
    // attributes
    Ok(res
        .add_attribute("action", "repay_loan")
        .add_attribute("borrower", borrower)
        .add_attribute("lender", offer_info.lender)
        .add_attribute("loan_id", loan_id.to_string())
        .add_attribute("interest", interests))
}

/// Withdraw the collateral from a defaulted loan
/// If the loan duration has exceeded, the collateral can be withdrawn by the lender
/// This closes the loan and puts it in a defaulted state
pub fn withdraw_defaulted_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    borrower: String,
    loan_id: u64,
) -> Result<Response, ContractError> {
    // We query the loan info
    let borrower = deps.api.addr_validate(&borrower)?;
    let mut collateral = COLLATERAL_INFO.load(deps.storage, (borrower.clone(), loan_id))?;
    is_loan_defaulted(deps.storage, env.clone(), &collateral)?;
    let offer = is_active_lender(deps.storage, info.sender, &collateral)?;

    // We need to test if the loan hasn't already been defaulted
    if collateral.state == LoanState::Defaulted {
        return Err(ContractError::LoanAlreadyDefaulted {});
    }

    // Saving the collateral state, the loan is defaulted, we can't default it again
    collateral.state = LoanState::Defaulted;
    COLLATERAL_INFO.save(deps.storage, (borrower.clone(), loan_id), &collateral)?;

    // We create the collateral withdrawal message
    let withdraw_messages = _withdraw_loan(collateral, env.contract.address, offer.lender.clone())?;

    Ok(Response::new()
        .add_messages(withdraw_messages)
        .add_attribute("action", "default_loan")
        .add_attribute("borrower", borrower)
        .add_attribute("lender", offer.lender)
        .add_attribute("loan_id", loan_id.to_string()))
}

pub fn _withdraw_loan(
    collateral: CollateralInfo,
    sender: Addr,
    recipient: Addr,
) -> StdResult<Vec<CosmosMsg>> {
    collateral
        .associated_assets
        .iter()
        .map(|collateral| _withdraw_asset(collateral, sender.clone(), recipient.clone()))
        .collect()
}

pub fn _withdraw_asset(asset: &AssetInfo, _sender: Addr, recipient: Addr) -> StdResult<CosmosMsg> {
    match asset {
        AssetInfo::Cw721Coin(cw721) => into_cosmos_msg(
            Cw721ExecuteMsg::TransferNft {
                recipient: recipient.to_string(),
                token_id: cw721.token_id.clone(),
            },
            cw721.address.clone(),
            None,
        ),
        #[cfg(not(feature = "vanilla"))]
        AssetInfo::Sg721Token(sg721) => into_cosmos_msg(
            Sg721ExecuteMsg::<Extension, Empty>::TransferNft {
                recipient: recipient.to_string(),
                token_id: sg721.token_id.clone(),
            },
            sg721.address.clone(),
            None,
        ),
        _ => Err(StdError::generic_err("msg")),
    }
}

// admin can lock contract
pub fn execute_toggle_lock(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    lock: bool,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // check the calling address is the authorised multisig
    ensure_eq!(info.sender, config.owner, ContractError::Unauthorized {});

    config.locks.lock = lock;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "sudo_update_status")
        .add_attribute("parameter", "contract_lock")
        .add_attribute("value", lock.to_string()))
}

// governance can lock contract
pub fn execute_sudo_toggle_lock(
    deps: DepsMut,
    _env: Env,
    lock: bool,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    config.locks.sudo_lock = lock;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "sudo_update_status")
        .add_attribute("parameter", "contract_lock")
        .add_attribute("value", lock.to_string()))
}
