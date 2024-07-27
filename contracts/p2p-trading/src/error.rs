use cosmwasm_std::{OverflowError, StdError};
use p2p_trading_export::state::TradeState;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("An unplanned bug just happened :/")]
    ContractBug {},

    #[error("Key already exists in TradeInfo")]
    ExistsInTradeInfo {},

    #[error("Key does not exist in TradeInfo")]
    NotFoundInTradeInfo {},

    #[error(
        "The trade_id field should be present when modifying the last counter_trade submitted"
    )]
    TradeIdMissing {},

    #[error("Trader not creator of the trade")]
    TraderNotCreator {},

    #[error("Key already exists in CounterTradeInfo")]
    ExistsInCounterTradeInfo {},

    #[error("Key does not exist in CounterTradeInfo")]
    NotFoundInCounterTradeInfo {},

    #[error("Trader not creator of the CounterTrade")]
    CounterTraderNotCreator {},

    #[error("Trade cannot be countered, it is not ready or is already cancelled/terminated")]
    NotCounterable {},

    #[error("Wrong state of the trade for the current operation : {state:?}")]
    WrongTradeState { state: TradeState },

    #[error("Can change the state of the trade from {from:?} to {to:?}")]
    CantChangeTradeState { from: TradeState, to: TradeState },

    #[error("Sorry, you can't accept a counter trade that is not published yet")]
    TradeAlreadyAccepted {},

    #[error("Sorry, the trade is published, you can't modify it. You can cancel it if you're not satisfied")]
    TradeAlreadyPublished {},

    #[error("Sorry, this trade is not accepted yet")]
    TradeNotAccepted {},

    #[error("Sorry, this trade is cancelled")]
    TradeCancelled {},

    #[error("Sorry, this trade is not cancelled")]
    TradeNotCancelled {},

    #[error("Assets were already withdrawn, don't try to scam the platform please")]
    TradeAlreadyWithdrawn {},

    #[error("Can change the state of the counter-trade from {from:?} to {to:?}")]
    CantChangeCounterTradeState { from: TradeState, to: TradeState },

    #[error("Sorry, you can't accept a counter trade that is not published yet")]
    CantAcceptNotPublishedCounter {},

    #[error("Sorry, the counter trade is published, you can't modify it. You can cancel it if you're not satisfied")]
    CounterTradeAlreadyPublished {},

    #[error("Sorry, the trade has to be refused or cancelled to withdraw your funds")]
    CounterTradeNotAborted {},

    #[error("Only the trader or the counter-trader can withdraw assets, don't try to scam the platform please")]
    NotWithdrawableByYou {},

    #[error("This trade is only allowed to a selected few, sorry :/")]
    AddressNotWhitelisted {},

    #[error(
        "Asset not found in your trade (wrong position or wrong asset specified or wrong token_id)"
    )]
    AssetNotFound { position: usize },

    #[error("Asset found in your trade but you are trying to withdraw too much. address: {address:?}, wanted: {wanted:?}, available {available:?}")]
    TooMuchWithdrawn {
        address: String,
        wanted: u128,
        available: u128,
    },

    #[error("You indicated the wrong token")]
    WrongTokenType {},

    #[error("You can't set a preview of an asset not associated with the trade")]
    AssetNotInTrade {},
}
