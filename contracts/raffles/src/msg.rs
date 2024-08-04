use crate::state::{
    CreateLocalityParams, FeeDiscount, FeeDiscountMsg, LocalityInfo, LocalityState, RaffleInfo, RaffleOptionsMsg, RaffleState
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, HexBinary, StdError, StdResult};
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
    /// Coins required for creating a raffle
    pub creation_coins: Option<Vec<Coin>>,
    /// Various discount scenarios for raffle participants
    pub fee_discounts: Vec<FeeDiscountMsg>,
    /// global params for locality instances
    pub locality_fee: Option<Decimal>,
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
    #[cw_orch(payable)]
    CreateRaffle {
        owner: Option<String>,
        assets: Vec<AssetInfo>,
        raffle_options: RaffleOptionsMsg,
        raffle_ticket_price: AssetInfo,
    },
    CancelRaffle {
        raffle_id: u64,
    },
    ClaimRaffle {
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
        fee_discounts: Option<Vec<FeeDiscountMsg>>,
        locality_config: Option<Decimal>,
    },
    ModifyRaffle {
        raffle_id: u64,
        raffle_ticket_price: Option<AssetInfo>,
        raffle_options: RaffleOptionsMsg,
    },
    #[cw_orch(payable)]
    BuyTicket {
        raffle_id: u64,
        ticket_count: u32,
        sent_assets: AssetInfo,
        on_behalf_of: Option<String>,
    },
    Receive(cw721::Cw721ReceiveMsg),
    NoisReceive {
        callback: NoisCallback,
    },
    #[cw_orch(payable)]
    CreateLocality {
        locality_params: CreateLocalityParams,
    },
    #[cw_orch(payable)]
    PurchaseLocalityTicket {
        id: u64,
        ticket_count: u32,
        assets: AssetInfo,
    },
    // Admin messages
    /// Provide job_id for randomness contract
    UpdateRandomness {
        raffle_id: u64,
    },
    ToggleLock {
        lock: bool,
    },
    ToggleLocality{
        on: bool,
    }
}

#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(FeeDiscountResponse)]
    FeeDiscount { user: String },
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
    pub gated_rights_ticket_buyer: Option<String>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub name: String,
    pub owner: String,
    pub fee_addr: String,
    pub last_raffle_id: u64,
    pub minimum_raffle_duration: u64, // The minimum interval in which users can buy raffle tickets
    pub max_tickets_per_raffle: Option<u32>,
    pub raffle_fee: Decimal, // The percentage of the resulting ticket-tokens that will go to the treasury
    pub locks: Locks,        // Wether the contract can accept new raffles
    pub nois_proxy_addr: String,
    pub nois_proxy_coin: Coin,
    pub creation_coins: Vec<Coin>,
    pub fee_discounts: Vec<FeeDiscount>,
}

#[cw_serde]
pub struct FeeDiscountResponse {
    pub discounts: Vec<(FeeDiscount, bool)>,
    pub total_discount: Decimal,
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
pub struct AllLocalitiesResponse {
    pub localities: Vec<LocalityResponse>,
}

#[cw_serde]
pub struct LocalityResponse {
    pub id: u64,
    pub state: LocalityState,
    pub info: LocalityInfo,
    pub frequency: u64,
}

#[cw_serde]
pub struct MigrateMsg {}
