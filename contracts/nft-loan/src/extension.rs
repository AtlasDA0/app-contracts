use crate::{
    error::ContractError,
    state::{
        can_repay_loan, is_active_lender, lender_offers, LoanExtensionInfo, COLLATERAL_INFO,
        LOAN_EXTENSION_INFO,
    },
};
use cosmwasm_std::{ensure_eq, DepsMut, Env, MessageInfo, StdError, Uint128};
use utils::{state::is_valid_comment, types::Response};

pub fn request_loan_extension(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    loan_id: u64,
    comment: Option<String>,
    additional_interest: Uint128,
    additional_duration: u64,
) -> Result<Response, ContractError> {
    let borrower = info.sender;
    let collateral = COLLATERAL_INFO.load(deps.storage, (borrower.clone(), loan_id))?;

    // We verify the loan is not defaulted
    can_repay_loan(deps.storage, env, &collateral)?;

    // checks comment size
    if !is_valid_comment(&comment.clone().unwrap_or_default()) {
        return Err(ContractError::Std(StdError::generic_err(
            "Comment too long. max = (20000 UTF-8 bytes)",
        )));
    }
    // We submit the extension with an id (to avoid front-running)
    LOAN_EXTENSION_INFO.update(
        deps.storage,
        (borrower.clone(), loan_id),
        |existing_extension| {
            let extension_id = match existing_extension {
                Some(existing_extension) => existing_extension.extension_id + 1,
                None => 0,
            };

            Ok::<_, ContractError>(LoanExtensionInfo {
                comment,
                extension_id,
                additional_interest,
                additional_duration,
            })
        },
    )?;

    // response attributes
    Ok(Response::new()
        .add_attribute("action", "request_extension")
        .add_attribute("borrower", borrower)
        .add_attribute("loan_id", loan_id.to_string()))
}

pub fn accept_loan_extension(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    borrower: String,
    loan_id: u64,
    extension_id: u32,
) -> Result<Response, ContractError> {
    let borrower = deps.api.addr_validate(&borrower)?;
    let collateral = COLLATERAL_INFO.load(deps.storage, (borrower.clone(), loan_id))?;
    let mut loan_offer = is_active_lender(deps.storage, info.sender, &collateral)?;

    // Lender can accept extension even if loan is defaulted

    // We make sure the extension is the one the lender wanted to accept
    let extension_info = LOAN_EXTENSION_INFO.load(deps.storage, (borrower.clone(), loan_id))?;
    ensure_eq!(
        extension_info.extension_id,
        extension_id,
        ContractError::WrongExtensionId {
            expected: extension_id,
            got: extension_info.extension_id
        }
    );

    // Lender accepts the extension, we update the loan offer that has been accepted
    loan_offer.terms.duration_in_blocks += extension_info.additional_duration;
    loan_offer.terms.interest += extension_info.additional_interest;

    lender_offers().save(deps.storage, &collateral.active_offer.unwrap(), &loan_offer)?;

    // We remove the loan extension
    LOAN_EXTENSION_INFO.remove(deps.storage, (borrower.clone(), loan_id));

    Ok(Response::new()
        .add_attribute("action", "accept_extension")
        .add_attribute("borrower", borrower)
        .add_attribute("loan_id", loan_id.to_string()))
}
