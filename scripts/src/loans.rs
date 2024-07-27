use cw_orch::prelude::*;
use nft_loans_nc::msg::*;

#[cw_orch::interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty, id = "loans")]
pub struct Loans;

impl<Chain: CwEnv> Uploadable for Loans<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("nft_loans_nc")
            .unwrap()
    }
    // No wrapper because there is custom msgs and queries,
    // Not supported by cw-orch
}
