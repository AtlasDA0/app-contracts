use cw_orch::{interface, prelude::*};

pub const CONTRACT_ID: &str = "raffles";
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct Raffles;

impl<Chain> Uploadable for Raffles<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("raffles")
            .unwrap()
    }
}
