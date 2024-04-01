use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    ensure, Addr, Api, Coin, Decimal, Env, HexBinary, StdError, StdResult, Storage, Timestamp,
    Uint128,
};
use cw20::{Cw20Coin, Cw20CoinVerified};
use cw_storage_plus::{Item, Map};
use utils::state::{AssetInfo, Locks};

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
pub const MAX_TICKET_NUMBER: u32 = 100000; // The maximum amount of tickets () that can be in a raffle
pub const MINIMUM_RAFFLE_DURATION: u64 = 1; // default minimum raffle duration, in seconds
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

    /// Fee bypass for Atlas Dao NFT holders
    pub atlas_dao_nft_address: Option<Addr>,

    /// Discount applied to stakers on fees (0.5 corresponds to paying only 50% treasury fees)
    /// This is not applied on royalty fees
    pub staker_fee_discount: StakerFeeDiscount,
}

#[cw_serde]
pub struct StakerFeeDiscount {
    pub discount: Decimal,
    pub minimum_amount: Uint128,
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
    pub owner: Addr,                    // owner/admin of the raffle
    pub assets: Vec<AssetInfo>,         // assets being raffled off
    pub raffle_ticket_price: AssetInfo, // cost per ticket
    pub number_of_tickets: u32,         // number of tickets purchased
    pub randomness: Option<HexBinary>,  // randomness seed provided by nois_proxy
    pub winners: Vec<Addr>,             // winner is determined here
    pub is_cancelled: bool,
    pub raffle_options: RaffleOptions,
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
    pub max_ticket_number: Option<u32>,    // max amount of tickets able to be purchased
    pub max_ticket_per_address: Option<u32>, // max amount of tickets able to bought per address
    pub raffle_preview: u32,               // ?
    pub min_ticket_number: Option<u32>,    // Minimum ticket number for a raffle to close.
    pub one_winner_per_asset: bool, // Allows to set multiple winners per raffle (one per asset)

    pub gating_raffle: Vec<GatingOptions>, // Allows for token gating raffle tickets. Only owners of those tokens can buy raffle tickets
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

#[cw_serde]
pub struct RaffleOptionsMsg {
    pub raffle_start_timestamp: Option<Timestamp>,
    pub raffle_duration: Option<u64>,
    pub comment: Option<String>,
    pub max_ticket_number: Option<u32>,
    pub max_ticket_per_address: Option<u32>,
    pub raffle_preview: Option<u32>,
    pub one_winner_per_asset: bool,
    pub min_ticket_number: Option<u32>,

    pub gating_raffle: Vec<GatingOptionsMsg>,
}

#[cw_serde]
pub enum GatingOptionsMsg {
    Cw721Coin(String),
    Cw20(Cw20Coin),
    Coin(Coin),
    Sg721Token(String),
    DaoVotingPower {
        dao_address: String,
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
    pub fn new(
        api: &dyn Api,
        env: Env,
        assets_len: usize,
        raffle_options: RaffleOptionsMsg,
        config: Config,
    ) -> StdResult<Self> {
        Ok(Self {
            raffle_start_timestamp: raffle_options
                .raffle_start_timestamp
                .unwrap_or(env.block.time)
                .max(env.block.time),
            raffle_duration: raffle_options
                .raffle_duration
                .unwrap_or(config.minimum_raffle_duration)
                .max(config.minimum_raffle_duration),
            comment: raffle_options.comment,
            max_ticket_number: if let Some(global_max) = config.max_tickets_per_raffle {
                if let Some(this_max) = raffle_options.max_ticket_number {
                    Some(global_max.min(this_max))
                } else {
                    Some(global_max)
                }
            } else {
                raffle_options.max_ticket_number
            },
            max_ticket_per_address: raffle_options.max_ticket_per_address,
            raffle_preview: raffle_options
                .raffle_preview
                .map(|preview| {
                    if preview >= assets_len.try_into().unwrap() {
                        0u32
                    } else {
                        preview
                    }
                })
                .unwrap_or(0u32),

            one_winner_per_asset: raffle_options.one_winner_per_asset,
            // We need to enforce a min ticket number in case we have one winner per asset
            // Because one ticket can't win more than one NFT
            min_ticket_number: if raffle_options.one_winner_per_asset {
                if let Some(min_ticket_number) = raffle_options.min_ticket_number {
                    Some(min_ticket_number.min(assets_len as u32))
                } else {
                    Some(assets_len as u32)
                }
            } else {
                raffle_options.min_ticket_number
            },

            gating_raffle: raffle_options
                .gating_raffle
                .into_iter()
                .map(|options| {
                    Ok::<_, StdError>(match options {
                        GatingOptionsMsg::Cw721Coin(address) => {
                            GatingOptions::Cw721Coin(api.addr_validate(&address)?)
                        }
                        GatingOptionsMsg::Coin(coin) => GatingOptions::Coin(coin),
                        GatingOptionsMsg::Sg721Token(address) => {
                            GatingOptions::Sg721Token(api.addr_validate(&address)?)
                        }
                        GatingOptionsMsg::DaoVotingPower {
                            dao_address,
                            min_voting_power,
                        } => GatingOptions::DaoVotingPower {
                            dao_address: api.addr_validate(&dao_address)?,
                            min_voting_power,
                        },
                        GatingOptionsMsg::Cw20(c) => GatingOptions::Cw20(Cw20CoinVerified {
                            address: api.addr_validate(&c.address)?,
                            amount: c.amount,
                        }),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub fn new_from(
        api: &dyn Api,
        current_options: RaffleOptions,
        assets_len: usize,
        raffle_options: RaffleOptionsMsg,
        config: Config,
    ) -> StdResult<Self> {
        Ok(Self {
            raffle_start_timestamp: raffle_options
                .raffle_start_timestamp
                .unwrap_or(current_options.raffle_start_timestamp)
                .max(current_options.raffle_start_timestamp),
            raffle_duration: raffle_options
                .raffle_duration
                .unwrap_or(current_options.raffle_duration)
                .max(config.minimum_raffle_duration),
            comment: raffle_options.comment.or(current_options.comment),
            max_ticket_number: raffle_options
                .max_ticket_number
                .or(current_options.max_ticket_number),
            max_ticket_per_address: raffle_options
                .max_ticket_per_address
                .or(current_options.max_ticket_per_address),
            raffle_preview: raffle_options
                .raffle_preview
                .map(|preview| {
                    if preview >= assets_len.try_into().unwrap() {
                        0u32
                    } else {
                        preview
                    }
                })
                .unwrap_or(current_options.raffle_preview),
            one_winner_per_asset: raffle_options.one_winner_per_asset,
            // We need to enforce a min ticket number in case we have one winner per asset
            // Because one ticket can't win more than one NFT
            min_ticket_number: if raffle_options.one_winner_per_asset {
                if let Some(min_ticket_number) = raffle_options.min_ticket_number {
                    Some(min_ticket_number.min(assets_len as u32))
                } else {
                    Some(assets_len as u32)
                }
            } else {
                raffle_options.min_ticket_number
            },

            gating_raffle: raffle_options
                .gating_raffle
                .into_iter()
                .map(|options| {
                    Ok::<_, StdError>(match options {
                        GatingOptionsMsg::Cw721Coin(address) => {
                            GatingOptions::Cw721Coin(api.addr_validate(&address)?)
                        }
                        GatingOptionsMsg::Coin(coin) => GatingOptions::Coin(coin),
                        GatingOptionsMsg::Sg721Token(address) => {
                            GatingOptions::Sg721Token(api.addr_validate(&address)?)
                        }
                        GatingOptionsMsg::DaoVotingPower {
                            dao_address,
                            min_voting_power,
                        } => GatingOptions::DaoVotingPower {
                            dao_address: api.addr_validate(&dao_address)?,
                            min_voting_power,
                        },
                        GatingOptionsMsg::Cw20(c) => GatingOptions::Cw20(Cw20CoinVerified {
                            address: api.addr_validate(&c.address)?,
                            amount: c.amount,
                        }),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}
