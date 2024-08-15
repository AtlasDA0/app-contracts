use cw_orch::prelude::*;
use cw_infuser::{contract::{execute, instantiate,query,reply}, msg::*};

#[cw_orch::interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty, id = "loans")]
pub struct CwInfuser;

impl<Chain: CwEnv> Uploadable for CwInfuser<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("cw_infuser")
            .unwrap()
    }
  /// Returns a CosmWasm contract wrapper
  fn wrapper() -> Box<dyn MockContract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(execute, instantiate, query,).with_reply(reply))
}
}
