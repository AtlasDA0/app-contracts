use cosmwasm_schema::write_api;

use cosmwasm_std::Empty;
use randomness::{VerifierExecuteMsg, VerifierQueryMsg};

fn main() {
    write_api! {
        instantiate: Empty,
        execute: VerifierExecuteMsg,
        query: VerifierQueryMsg,
    }
}
