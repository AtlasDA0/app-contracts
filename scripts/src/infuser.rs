use cw_orch::prelude::*;
use cw_infuser::msg::*;

#[cw_orch::interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty, id = "loans")]
pub struct CwInfuser;

impl<Chain: CwEnv> Uploadable for CwInfuser<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("cw_infuser")
            .unwrap()
    }
    // No wrapper because there is custom msgs and queries,
    // Not supported by cw-orch
}
