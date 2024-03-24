use crate::state::{RaffleInfo, RaffleOptionsMsg, RaffleState};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, HexBinary, StdError, StdResult};
use nois::NoisCallback;
use utils::state::{is_valid_name, AssetInfo, Locks};

#[cw_serde]
pub struct InstantiateMsg {
    // Name of the raffle contract
    pub name: String,
    // Address of the nois_proxy
    pub nois_proxy_addr: String,
    // Coin expected for the randomness source
    pub nois_proxy_coin: Coin,
    // Admin of Contract
    pub owner: Option<String>,
    // Destination of Fee Streams
    pub fee_addr: Option<String>,
    // Minimum lifecycle length of raffle
    pub minimum_raffle_duration: Option<u64>,
    // Maximum participant limit for a raffle
    pub max_ticket_number: Option<u32>,
    // % fee of raffle ticket sales to fee_addr
    pub raffle_fee: Decimal,

    pub creation_coins: Option<Vec<Coin>>,
}

impl InstantiateMsg {
    pub fn validate(&self) -> StdResult<()> {
        // Check name
        if !is_valid_name(&self.name) {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }

        // Check the fee distribution
        if self.raffle_fee >= Decimal::one() {
            return Err(StdError::generic_err("The Fee rate should be lower than 1"));
        }

        Ok(())
    }
}

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
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
        max_tickets_per_raffle: Option<u32>,
        raffle_fee: Option<Decimal>,
        nois_proxy_addr: Option<String>,
        nois_proxy_coin: Option<Coin>,
        creation_coins: Option<Vec<Coin>>,
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
    NoisReceive {
        callback: NoisCallback,
    },

    // Admin messages
    /// Provide job_id for randomness contract
    UpdateRandomness {
        raffle_id: u64,
    },
    ToggleLock {
        lock: bool,
    },
}

#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
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
    TicketCount { owner: String, raffle_id: u64 },
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
    pub raffle_fee: Decimal, // The percentage of the resulting ticket-tokens that will go to the treasury
    pub locks: Locks,        // Wether the contract can accept new raffles
    pub nois_proxy_addr: Addr,
    pub nois_proxy_coin: Coin,
    pub creation_coins: Vec<Coin>,
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
