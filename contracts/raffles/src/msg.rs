use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, HexBinary, StdError, StdResult, Uint128};
use nois::NoisCallback;
use utils::state::AssetInfo;

use crate::state::{RaffleInfo, RaffleOptionsMsg, RaffleState};

#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub nois_proxy_addr: String,
    pub nois_proxy_denom: String,
    pub nois_proxy_amount: Uint128,
    pub creation_fee_denom: Option<Vec<String>>,
    pub creation_fee_amount: Option<Uint128>,
    pub owner: Option<String>,
    pub fee_addr: Option<String>, 
    pub minimum_raffle_duration: Option<u64>,
    pub minimum_raffle_timeout: Option<u64>,
    pub max_participant_number: Option<u32>,
    pub raffle_fee: Option<Decimal>,
    pub rand_fee: Option<Decimal>,
}

impl InstantiateMsg {
    pub fn validate(&self) -> StdResult<()> {
        // Check name
        if !is_valid_name(&self.name) {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        Ok(())
    }
}

fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 50 {
        return false;
    }
    true
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateRaffle {
        owner: Option<String>,
        assets: Vec<AssetInfo>,
        raffle_options: RaffleOptionsMsg,
        raffle_ticket_price: AssetInfo,
    },
    CancelRaffle {
        raffle_id: u64,
    },
    UpdateConfig {
        name: Option<String>,
        owner: Option<String>,
        fee_addr: Option<String>,
        minimum_raffle_duration: Option<u64>,
        minimum_raffle_timeout: Option<u64>,
        creation_fee_denom: Option<Vec<String>>,
        creation_fee_amount: Option<Uint128>,
        raffle_fee: Option<Decimal>,
        nois_proxy_addr: Option<String>,
        nois_proxy_denom: Option<String>,
        nois_proxy_amount: Option<Uint128>,
    },
    ModifyRaffle {
        raffle_id: u64,
        raffle_ticket_price: Option<AssetInfo>,
        raffle_options: RaffleOptionsMsg,
    },
    BuyTicket {
        raffle_id: u64,
        ticket_count: u32,
        sent_assets: AssetInfo,
    },
    Receive(cw721::Cw721ReceiveMsg),
    ClaimNft {
        raffle_id: u64,
    },
    NoisReceive {
        callback: NoisCallback,
        raffle_id: u64,
    },
    // Admin messages
    ToggleLock {
        lock: bool,
    },
    // provide job_id for randomness contract
    UpdateRandomness {
        raffle_id: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(RaffleResponse)]
    RaffleInfo { raffle_id: u64 },
    #[returns(AllRafflesResponse)]
    AllRaffles {
        start_after: Option<u64>,
        limit: Option<u32>,
        filters: Option<QueryFilters>,
    },
    #[returns(Vec<String>)]
    AllTickets {
        raffle_id: u64,
        start_after: Option<u32>,
        limit: Option<u32>,
    },
    #[returns(u32)]
    TicketNumber { owner: String, raffle_id: u64 },
}

#[cw_serde]
pub struct QueryFilters {
    pub states: Option<Vec<String>>,
    pub owner: Option<String>,
    pub ticket_depositor: Option<String>,
    pub contains_token: Option<String>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub name: String,
    pub owner: Addr,
    pub fee_addr: Addr,
    pub last_raffle_id: u64,
    pub minimum_raffle_duration: u64, // The minimum interval in which users can buy raffle tickets
    pub minimum_raffle_timeout: u64, // The minimum interval during which users can provide entropy to the contract
    pub creation_fee_amount: Uint128,
    pub creation_fee_denom: Vec<String>,
    pub raffle_fee: Decimal, // The percentage of the resulting ticket-tokens that will go to the treasury
    pub lock: bool,          // Wether the contract can accept new raffles
    pub nois_proxy_addr: Addr,
    pub nois_proxy_denom: String,
    pub nois_proxy_amount: Uint128,
}

#[cw_serde]
pub struct RaffleResponse {
    pub raffle_id: u64,
    pub raffle_state: RaffleState,
    pub raffle_info: Option<RaffleInfo>,
}

#[cw_serde]
pub struct AllRafflesResponse {
    pub raffles: Vec<RaffleResponse>,
}

#[cw_serde]
pub struct IsLuckyResponse {
    pub is_lucky: Option<bool>,
}

#[cw_serde]
pub struct MerkleRootResponse {
    /// MerkleRoot is hex-encoded merkle root.
    pub merkle_root: HexBinary,
}

#[cw_serde]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[cw_serde]
pub struct MigrateMsg {}
