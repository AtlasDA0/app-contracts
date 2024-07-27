pub mod contract;
pub mod counter_trade;
mod error;
pub mod messages;
pub mod query;
pub mod state;
pub mod trade;

pub use crate::error::ContractError;

#[cfg(not(target_arch = "wasm32"))]
mod interface;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::interface::P2PTrading;
