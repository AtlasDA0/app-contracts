use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};

#[cw_serde]
pub struct DrandRandomness {
    pub round: u64,
    pub previous_signature: Binary,
    pub signature: Binary,
}

#[cw_serde]
pub struct Randomness {
    pub randomness: [u8; 32],
    pub randomness_round: u64,
    pub randomness_owner: Addr,
}

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum VerifierExecuteMsg {
    Verify {
        randomness: DrandRandomness,
        pubkey: Binary,
        raffle_id: u64,
        owner: String,
    },
}

#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum VerifierQueryMsg {}
