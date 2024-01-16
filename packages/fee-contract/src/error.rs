use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Fee not paid correctly, required: {required:?}uust, provided {provided:?}uust")]
    FeeNotPaidCorrectly { required: u128, provided: u128 },

    #[error("Fee not paid")]
    FeeNotPaid {},

    #[error("Trade not accepted")]
    TradeNotAccepted {},

    #[error("Fee tiers not ordered, you can't change them")]
    TiersNotOrdered {},

    #[error("Error when encoding response message to binary string")]
    BinaryEncodingError {},
}
