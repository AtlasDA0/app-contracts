use crate::state::{GatingOptions, RaffleInfo, RaffleOptions, RaffleState, TicketOptions, CONFIG};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    Addr, Coin, Decimal, Deps, Env, HexBinary, StdError, StdResult, Timestamp, Uint128,
};
use cw20::{Cw20Coin, Cw20CoinVerified};
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
pub enum ExecuteMsg {
    CreateRaffle {
        owner: Option<String>,
        assets: Vec<AssetInfo>,
        raffle_options: RaffleOptionsMsg,
        ticket_options: TicketOptionsMsg,
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
        raffle_options: RaffleOptionsMsg,
        ticket_options: TicketOptionsMsg,
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

#[cw_serde]
pub struct RaffleOptionsMsg {
    pub raffle_start_timestamp: Option<Timestamp>,
    pub raffle_duration: Option<u64>,
    pub comment: Option<String>,
    pub raffle_preview: Option<u32>,
}

impl RaffleOptionsMsg {
    pub fn check(self, deps: Deps, env: Env, assets_len: usize) -> Result<RaffleOptions, StdError> {
        let config = CONFIG.load(deps.storage)?;
        Ok(RaffleOptions {
            raffle_start_timestamp: self
                .raffle_start_timestamp
                .unwrap_or(env.block.time)
                .max(env.block.time),
            raffle_duration: self
                .raffle_duration
                .unwrap_or(config.minimum_raffle_duration)
                .max(config.minimum_raffle_duration),
            comment: self.comment,
            raffle_preview: self
                .raffle_preview
                .map(|preview| {
                    if preview >= assets_len.try_into().unwrap() {
                        0u32
                    } else {
                        preview
                    }
                })
                .unwrap_or(0u32),
        })
    }
}

#[cw_serde]
pub struct TicketOptionsMsg {
    pub raffle_ticket_price: AssetInfo,
    pub max_ticket_number: Option<u32>,
    pub max_ticket_per_address: Option<u32>,
    pub min_ticket_number: Option<u32>,
    pub gating: Vec<GatingOptionsMsg>,
    pub one_winner_per_asset: bool,
}

impl TicketOptionsMsg {
    pub fn check(self, deps: Deps, assets_len: usize) -> Result<TicketOptions, StdError> {
        let config = CONFIG.load(deps.storage)?;
        Ok(TicketOptions {
            max_ticket_number: if let Some(global_max) = config.max_tickets_per_raffle {
                if let Some(this_max) = self.max_ticket_number {
                    Some(global_max.min(this_max))
                } else {
                    Some(global_max)
                }
            } else {
                self.max_ticket_number
            },
            max_ticket_per_address: self.max_ticket_per_address,

            one_winner_per_asset: self.one_winner_per_asset,
            min_ticket_number: if self.one_winner_per_asset {
                if let Some(min_ticket_number) = self.min_ticket_number {
                    Some(min_ticket_number.min(assets_len as u32))
                } else {
                    Some(assets_len as u32)
                }
            } else {
                self.min_ticket_number
            },

            gating: self
                .gating
                .into_iter()
                .map(|options| options.check(deps))
                .collect::<Result<Vec<_>, _>>()?,
            raffle_ticket_price: self.raffle_ticket_price,
        })
    }
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

impl GatingOptionsMsg {
    pub fn check(self, deps: Deps) -> Result<GatingOptions, StdError> {
        Ok(match self {
            GatingOptionsMsg::Cw721Coin(address) => {
                GatingOptions::Cw721Coin(deps.api.addr_validate(&address)?)
            }
            GatingOptionsMsg::Coin(coin) => GatingOptions::Coin(coin),
            GatingOptionsMsg::Sg721Token(address) => {
                GatingOptions::Sg721Token(deps.api.addr_validate(&address)?)
            }
            GatingOptionsMsg::DaoVotingPower {
                dao_address,
                min_voting_power,
            } => GatingOptions::DaoVotingPower {
                dao_address: deps.api.addr_validate(&dao_address)?,
                min_voting_power,
            },
            GatingOptionsMsg::Cw20(c) => GatingOptions::Cw20(Cw20CoinVerified {
                address: deps.api.addr_validate(&c.address)?,
                amount: c.amount,
            }),
        })
    }
}
