use cw_orch::{interface, prelude::*};

pub const CONTRACT_ID: &str = "verifier";
use crate::contract::VerifierExecuteMsg;

#[interface(Empty, VerifierExecuteMsg, Empty, Empty, id = CONTRACT_ID)]
pub struct Verifier;

impl<Chain> Uploadable for Verifier<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("randomness_verifier")
            .unwrap()
    }

    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        ))
    }
}
