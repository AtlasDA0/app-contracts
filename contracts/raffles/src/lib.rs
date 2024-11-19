#![allow(clippy::result_large_err)]

pub mod contract;
pub mod error;
pub mod execute;
pub mod msg;
pub mod query;
pub mod randomness;
pub mod state;
pub mod utils;

#[cfg(not(target_arch = "wasm32"))]
mod interface;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::interface::Raffles;
