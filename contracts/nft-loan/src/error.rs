use cosmwasm_std::{CoinsError, StdError, Uint128};
use thiserror::Error;

use crate::state::{LoanState, OfferState};

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Coins(#[from] CoinsError),

    #[error("Unreachable error")]
    Unreachable {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("An unplanned bug just happened :/")]
    ContractBug {},

    #[error("This action is not allowed, the contract is locked")]
    ContractIsLocked {},

    #[error("Wrong asset deposited, only cw1155 and cw721 are authorized")]
    WrongAssetDeposited {},

    #[error("Please include at least one asset when creating a loan")]
    NoAssets {},

    #[error("You need to send exactly one coin with this transaction")]
    MultipleCoins {},

    #[error("Fund sent do not match the loan terms")]
    FundsDontMatchTerms {},

    #[error("Fund sent do not match the loan terms, {0}, {1}")]
    FundsDontMatchTermsAndPrinciple(Uint128, Uint128),

    #[error("Sorry, your asset is not withdrawable at this stage")]
    NotWithdrawable {},

    #[error("Sorry, your asset is not withdrawable at this stage")]
    NotModifiable {},

    #[error("Sorry, no assets to withdraw here")]
    NoFundsToWithdraw {},

    #[error("The Message sender has to be the owner of the NFT to prevent hacks")]
    SenderNotOwner {},

    #[error("Sorry, you can't accept this loan")]
    NotAcceptable {},

    #[error("The fee_rate you provided is not greater than 0, or less than 1")]
    InvalidFeeRate {},

    #[error("Sorry, you can't make an offer on this loan")]
    NotCounterable {},

    #[error("Sorry, you can't refuse this offer, it's not published")]
    NotRefusable {},

    #[error("This loan doesn't have any terms")]
    NoTermsSpecified {},

    #[error("Sorry, this loan doesn't exist :/")]
    LoanNotFound {},

    #[error("Sorry, this offer doesn't exist :/")]
    OfferNotFound {},

    #[error("Wrong state of the loan for the current operation : {state:?}")]
    WrongLoanState { state: LoanState },

    #[error("Wrong state of the offer for the current operation : {state:?}")]
    WrongOfferState { state: OfferState },

    #[error("Can change the state of the offer from {from:?} to {to:?}")]
    CantChangeOfferState { from: OfferState, to: OfferState },

    #[error("The loan has already been defaulted, you can't withdraw the funds again")]
    LoanAlreadyDefaulted {},

    #[error("You can't set a preview of an asset not associated with the loan")]
    AssetNotInLoan {},

    #[error("Invalid Amount")]
    InvalidAmount {},

    #[error("You did not provide the required fee to request for a loan")]
    DepositFeeError {},

    #[error("There is an issue with the approval of the tokens. please approve this contract to interact with your nfts & try again!")]
    TokenApprovalIssue {},

    #[error("Invalid Name")]
    InvalidName {},
}
