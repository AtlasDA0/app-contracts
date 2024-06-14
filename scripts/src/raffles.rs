use cw_orch::prelude::*;
use raffles::msg::*;

#[cw_orch::interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = "raffles")]
pub struct Raffles;

impl<Chain: CwEnv> Uploadable for Raffles<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(&self) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("raffles")
            .unwrap()
    }
    // No wrapper because there is custom msgs and queries,
    // Not supported by cw-orch
}
