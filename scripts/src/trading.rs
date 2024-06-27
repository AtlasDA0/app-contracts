use cw_orch::prelude::*;
use p2p_trading_export::msg::*;

#[cw_orch::interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = "trading")]
pub struct Trading;

impl<Chain: CwEnv> Uploadable for Trading<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("p2p_trading")
            .unwrap()
    }
    // No wrapper because there is custom msgs and queries,
    // Not supported by cw-orch
}
