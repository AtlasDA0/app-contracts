use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    ensure, Addr, Coin, Decimal, Deps, Env, HexBinary, StdError, StdResult, Storage, Timestamp,
    Uint128,
};
use cw20::{Cw20Coin, Cw20CoinVerified};
use cw_storage_plus::{Item, Map};
use utils::state::{AssetInfo, Locks};

use crate::msg::{GatingOptionsMsg, RaffleOptionsMsg, TicketOptionsMsg};

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
pub const MAX_TICKET_NUMBER: u32 = 100000; // The maximum amount of tickets () that can be in a raffle
pub const MINIMUM_RAFFLE_DURATION: u64 = 1; // default minimum raffle duration, in blocks
pub const RAFFLE_INFO: Map<u64, RaffleInfo> = Map::new("raffle_info");
pub const RAFFLE_TICKETS: Map<(u64, u32), Addr> = Map::new("raffle_tickets");
pub const STATIC_RAFFLE_CREATION_FEE: u128 = 100; // default static tokens required to create raffle
pub const USER_TICKETS: Map<(&Addr, u64), u32> = Map::new("user_tickets");

#[cw_serde]
pub struct Config {
    /// The name of the contract
    pub name: String,
    /// The owner address of the contract
    pub owner: Addr,
    /// The address to recieve all fees generated by the contract
    pub fee_addr: Addr,
    /// The most recent raffle id
    pub last_raffle_id: Option<u64>,
    /// The minimum duration, in seconds, in which users can buy raffle tickets
    pub minimum_raffle_duration: u64,
    // The maximum number of participants available to participate in any 1 raffle
    pub max_tickets_per_raffle: Option<u32>,
    /// A % cut of all raffle fee's generated to go to the fee_addr
    pub raffle_fee: Decimal,
    /// locks the contract from new raffles being created
    pub locks: Locks,
    /// The nois_proxy contract address
    pub nois_proxy_addr: Addr,
    /// The expected fee token denomination of the nois_proxy contract
    pub nois_proxy_coin: Coin,
    pub creation_coins: Vec<Coin>,
}

impl Config {
    pub fn validate_fee(&self) -> Result<(), StdError> {
        ensure!(
            self.raffle_fee <= Decimal::one(),
            StdError::generic_err("The Total Fee rate should be lower than 1")
        );
        Ok(())
    }
}

#[cw_serde]
pub struct NoisProxy {
    // The price to pay the proxy for randomness
    pub price: Coin,
    // The address of the nois-proxy contract deployed onthe same chain as this contract
    pub address: Addr,
}

// RAFFLES

pub fn load_raffle(storage: &dyn Storage, raffle_id: u64) -> StdResult<RaffleInfo> {
    RAFFLE_INFO.load(storage, raffle_id)
}

#[cw_serde]
pub struct RaffleInfo {
    pub owner: Addr,                   // owner/admin of the raffle
    pub assets: Vec<AssetInfo>,        // assets being raffled off
    pub number_of_tickets: u32,        // number of tickets purchased
    pub randomness: Option<HexBinary>, // randomness seed provided by nois_proxy
    pub winners: Vec<Addr>,            // winner is determined here
    pub is_cancelled: bool,
    pub raffle_options: RaffleOptions,
    pub ticket_options: TicketOptions,
}

#[cw_serde]
pub enum RaffleState {
    Created,
    Started,
    Closed,
    Claimed,
    Cancelled,
}

impl std::fmt::Display for RaffleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RaffleState::Created => write!(f, "created"),
            RaffleState::Started => write!(f, "started"),
            RaffleState::Closed => write!(f, "closed"),
            RaffleState::Claimed => write!(f, "claimed"),
            RaffleState::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Queries the raffle state
/// This function depends on the block time to return the RaffleState.
/// As actions can only happen in certain time-periods, you have to be careful when testing off-chain
/// If the chains stops or the block time is not accurate we might get some errors (let's hope it never happens)
pub fn get_raffle_state(env: Env, raffle_info: &RaffleInfo) -> RaffleState {
    if raffle_info.is_cancelled {
        RaffleState::Cancelled
    } else if env.block.time < raffle_info.raffle_options.raffle_start_timestamp {
        RaffleState::Created
    } else if env.block.time
        < raffle_info
            .raffle_options
            .raffle_start_timestamp
            .plus_seconds(raffle_info.raffle_options.raffle_duration)
    {
        RaffleState::Started
    } else if raffle_info.randomness.is_none() {
        RaffleState::Closed
    } else {
        RaffleState::Claimed
    }
}

#[cw_serde]
pub struct RaffleOptions {
    pub raffle_start_timestamp: Timestamp, // If not specified, starts immediately
    pub raffle_duration: u64,              // length, in seconds the duration of a raffle
    pub comment: Option<String>,           // raffle description
    pub raffle_preview: u32,               // ?
}

#[cw_serde]
pub struct TicketOptions {
    pub raffle_ticket_price: AssetInfo,
    pub max_ticket_number: Option<u32>, // max amount of tickets able to be purchased
    pub max_ticket_per_address: Option<u32>, // max amount of tickets able to bought per address
    pub min_ticket_number: Option<u32>, // Minimum ticket number for a raffle to close.
    pub gating: Vec<GatingOptions>, // Allows for token gating raffle tickets. Only owners of those tokens can buy raffle tickets
    pub one_winner_per_asset: bool, // Allows to set multiple winners per raffle (one per asset)
}

#[cw_serde]
pub enum GatingOptions {
    Cw721Coin(Addr),
    Cw20(Cw20CoinVerified), // Corresponds to a minimum of coins that a raffle buyer should own
    Coin(Coin),             // Corresponds to a minimum of coins that a raffle buyer should own
    Sg721Token(Addr),
    DaoVotingPower {
        dao_address: Addr,
        min_voting_power: Uint128,
    },
}

impl From<GatingOptions> for GatingOptionsMsg {
    fn from(options: GatingOptions) -> GatingOptionsMsg {
        match options {
            GatingOptions::Cw721Coin(address) => GatingOptionsMsg::Cw721Coin(address.to_string()),
            GatingOptions::Coin(coin) => GatingOptionsMsg::Coin(coin),
            GatingOptions::Sg721Token(address) => GatingOptionsMsg::Sg721Token(address.to_string()),
            GatingOptions::DaoVotingPower {
                dao_address,
                min_voting_power,
            } => GatingOptionsMsg::DaoVotingPower {
                dao_address: dao_address.to_string(),
                min_voting_power,
            },
            GatingOptions::Cw20(c) => GatingOptionsMsg::Cw20(Cw20Coin {
                address: c.to_string(),
                amount: c.amount,
            }),
        }
    }
}

impl RaffleOptions {
    pub fn update(
        &self,
        deps: Deps,
        env: Env,
        assets_len: usize,
        mut raffle_options: RaffleOptionsMsg,
    ) -> StdResult<Self> {
        raffle_options.raffle_start_timestamp = raffle_options
            .raffle_start_timestamp
            .or(Some(self.raffle_start_timestamp));

        raffle_options.raffle_duration = raffle_options
            .raffle_duration
            .or(Some(self.raffle_duration));

        raffle_options.comment = raffle_options.comment.or(self.comment.clone());
        raffle_options.raffle_preview = raffle_options.raffle_preview.or(Some(self.raffle_preview));

        raffle_options.check(deps, env, assets_len)
    }
}
impl TicketOptions {
    pub fn update(
        &self,
        deps: Deps,
        assets_len: usize,
        mut ticket_options: TicketOptionsMsg,
    ) -> StdResult<Self> {
        ticket_options.max_ticket_number =
            ticket_options.max_ticket_number.or(self.max_ticket_number);
        ticket_options.max_ticket_per_address = ticket_options
            .max_ticket_per_address
            .or(self.max_ticket_per_address);
        ticket_options.min_ticket_number =
            ticket_options.min_ticket_number.or(self.min_ticket_number);

        let options = ticket_options.check(deps, assets_len)?;

        Ok(options)
    }
}
