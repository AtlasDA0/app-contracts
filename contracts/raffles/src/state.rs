use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    ensure, ensure_eq, Addr, Api, Coin, Decimal, Deps, Empty, Env, HexBinary, StdError, StdResult,
    Storage, Timestamp, Uint128,
};
use cw20::{Cw20Coin, Cw20CoinVerified};
use cw_storage_plus::{Item, Map};
use dao_interface::voting::VotingPowerAtHeightResponse;
use randomness::Randomness;
use utils::state::{AssetInfo, Locks};

use crate::{error::ContractError, msg::DrandConfig};

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
pub const OLD_CONFIG: Item<OldConfig> = Item::new(CONFIG_KEY);
pub const MAX_TICKET_NUMBER: u32 = 100000; // The maximum amount of tickets () that can be in a raffle
pub const MINIMUM_RAFFLE_DURATION: u64 = 1; // default minimum raffle duration, in seconds
pub const RAFFLE_INFO: Map<u64, RaffleInfo> = Map::new("raffle_info");
pub const RAFFLE_TICKETS: Map<(u64, u32), Addr> = Map::new("raffle_tickets");
pub const STATIC_RAFFLE_CREATION_FEE: u128 = 100; // default static tokens required to create raffle
pub const USER_TICKETS: Map<(&Addr, u64), u32> = Map::new("user_tickets");

pub const NFT_TOKEN_LIMIT: u32 = 20;

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
    pub creation_coins: Vec<Coin>,

    /// Fee discounts applied
    pub fee_discounts: Vec<FeeDiscount>,

    pub drand_config: DrandConfig,
}

#[cw_serde]
pub struct OldConfig {
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

    /// Fee discounts applied
    pub fee_discounts: Vec<FeeDiscount>,
}

#[cw_serde]
pub struct FeeDiscount {
    pub discount: Decimal,
    pub condition: AdvantageOptions,
}

impl From<FeeDiscount> for FeeDiscountMsg {
    fn from(options: FeeDiscount) -> FeeDiscountMsg {
        FeeDiscountMsg {
            discount: options.discount,
            condition: options.condition.into(),
        }
    }
}

impl FeeDiscountMsg {
    pub fn check(self, api: &dyn Api) -> StdResult<FeeDiscount> {
        ensure!(
            self.discount <= Decimal::one(),
            StdError::generic_err("Discount should be lower than 100%")
        );
        match &self.condition {
            AdvantageOptionsMsg::Cw721Coin {
                nft_count: _,
                nft_address: _,
            } => {}
            AdvantageOptionsMsg::Cw20(_) => {}
            AdvantageOptionsMsg::Coin(_) => {}
            AdvantageOptionsMsg::Sg721Token {
                nft_count: _,
                nft_address: _,
            } => {}
            AdvantageOptionsMsg::DaoVotingPower { .. } => {}
            AdvantageOptionsMsg::Staking { .. } => {}
        }
        Ok(FeeDiscount {
            discount: self.discount,
            condition: self.condition.check(api)?,
        })
    }
}

impl FeeDiscount {
    pub fn discount(&self, deps: Deps, user: String) -> Result<Decimal, ContractError> {
        let discount = match &self.condition {
            AdvantageOptions::Cw721Coin {
                nft_count,
                nft_address,
            } => {
                let owner_query: cw721::TokensResponse = deps.querier.query_wasm_smart(
                    nft_address,
                    &sg721_base::QueryMsg::Tokens {
                        owner: user.clone(),
                        start_after: None,
                        limit: Some(NFT_TOKEN_LIMIT),
                    },
                )?;
                self.discount
                    * Decimal::from_ratio(owner_query.tokens.len() as u32 / nft_count, 1u128)
            }
            AdvantageOptions::Cw20(_) => self.discount,
            AdvantageOptions::Coin(_) => self.discount,
            AdvantageOptions::Sg721Token {
                nft_count,
                nft_address,
            } => {
                let owner_query: cw721::TokensResponse = deps.querier.query_wasm_smart(
                    nft_address,
                    &sg721_base::QueryMsg::Tokens {
                        owner: user.clone(),
                        start_after: None,
                        limit: Some(NFT_TOKEN_LIMIT),
                    },
                )?;
                self.discount
                    * Decimal::from_ratio(owner_query.tokens.len() as u32 / nft_count, 1u128)
            }
            AdvantageOptions::DaoVotingPower {
                dao_address: _,
                min_voting_power: _,
            } => self.discount,
            AdvantageOptions::Staking {
                min_voting_power: _,
            } => self.discount,
        };
        Ok(discount.min(Decimal::one()))
    }
}

#[cw_serde]
pub struct FeeDiscountMsg {
    pub discount: Decimal,
    pub condition: AdvantageOptionsMsg,
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
    pub drand_randomness: Option<Randomness>, // This for drand now, migrating away from nois
}

#[cw_serde]
pub enum RaffleState {
    Created,
    Started,
    Closed,
    Claimed,
    Finished,
    Cancelled,
}

impl std::fmt::Display for RaffleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RaffleState::Created => write!(f, "created"),
            RaffleState::Started => write!(f, "started"),
            RaffleState::Closed => write!(f, "closed"),
            RaffleState::Claimed => write!(f, "claimed"),
            RaffleState::Finished => write!(f, "finished"),
            RaffleState::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Queries the raffle state
/// This function depends on the block time to return the RaffleState.
/// As actions can only happen in certain time-periods, you have to be careful when testing off-chain
/// If the chains stops or the block time is not accurate we might get some errors (let's hope it never happens)
pub fn get_raffle_state(env: &Env, config: &Config, raffle_info: &RaffleInfo) -> RaffleState {
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
    } else if env.block.time
        < raffle_info
            .raffle_options
            .raffle_start_timestamp
            .plus_seconds(raffle_info.raffle_options.raffle_duration)
            .plus_seconds(config.drand_config.timeout)
        || raffle_info.drand_randomness.is_none()
    {
        RaffleState::Closed
    } else if raffle_info.winners.is_empty() {
        RaffleState::Finished
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
    pub whitelist: Option<Vec<Addr>>,

    pub gating_raffle: Vec<AdvantageOptions>, // Allows for token gating raffle tickets. Only owners of those tokens can buy raffle tickets
}

impl From<RaffleOptions> for RaffleOptionsMsg {
    fn from(value: RaffleOptions) -> Self {
        Self {
            raffle_start_timestamp: Some(value.raffle_start_timestamp),
            raffle_duration: Some(value.raffle_duration),
            comment: value.comment,
            max_ticket_number: value.max_ticket_number,
            max_ticket_per_address: value.max_ticket_per_address,
            raffle_preview: Some(value.raffle_preview),
            one_winner_per_asset: value.one_winner_per_asset,
            min_ticket_number: value.min_ticket_number,
            gating_raffle: value.gating_raffle.into_iter().map(Into::into).collect(),
            whitelist: value
                .whitelist
                .map(|v| v.into_iter().map(Into::into).collect()),
        }
    }
}

#[cw_serde]
pub enum AdvantageOptions {
    Cw721Coin {
        nft_count: u32,
        nft_address: Addr,
    },
    Cw20(Cw20CoinVerified), // Corresponds to a minimum of coins that a raffle buyer should own
    Coin(Coin),             // Corresponds to a minimum of coins that a raffle buyer should own
    Sg721Token {
        nft_count: u32,
        nft_address: Addr,
    },
    DaoVotingPower {
        dao_address: Addr,
        min_voting_power: Uint128,
    },
    Staking {
        min_voting_power: Uint128,
    },
}

impl AdvantageOptions {
    pub fn has_advantage(&self, deps: Deps, user: String) -> Result<(), ContractError> {
        match self {
            crate::state::AdvantageOptions::Cw721Coin {
                nft_count,
                nft_address,
            } => {
                let owner_query: cw721::TokensResponse = deps.querier.query_wasm_smart(
                    nft_address,
                    &cw721_base::QueryMsg::<Empty>::Tokens {
                        owner: user.clone(),
                        start_after: None,
                        limit: Some(NFT_TOKEN_LIMIT),
                    },
                )?;
                ensure_eq!(
                    owner_query.tokens.len(),
                    *nft_count as usize,
                    ContractError::NotGatingCondition {
                        condition: self.clone(),
                        user
                    }
                );
                Ok::<_, ContractError>(())
            }
            crate::state::AdvantageOptions::Coin(needed_coins) => {
                // We verify the sender has enough coins in their wallet
                let user_balance = deps
                    .querier
                    .query_balance(user.clone(), needed_coins.denom.clone())?;

                ensure!(
                    user_balance.amount >= needed_coins.amount,
                    ContractError::NotGatingCondition {
                        condition: self.clone(),
                        user
                    }
                );
                Ok(())
            }
            crate::state::AdvantageOptions::Sg721Token {
                nft_count,
                nft_address,
            } => {
                let owner_query: cw721::TokensResponse = deps.querier.query_wasm_smart(
                    nft_address,
                    &sg721_base::QueryMsg::Tokens {
                        owner: user.clone(),
                        start_after: None,
                        limit: Some(NFT_TOKEN_LIMIT),
                    },
                )?;
                ensure_eq!(
                    owner_query.tokens.len(),
                    *nft_count as usize,
                    ContractError::NotGatingCondition {
                        condition: self.clone(),
                        user
                    }
                );
                Ok::<_, ContractError>(())
            }
            crate::state::AdvantageOptions::DaoVotingPower {
                dao_address,
                min_voting_power,
            } => {
                let voting_power: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
                    dao_address,
                    &dao_interface::msg::QueryMsg::VotingPowerAtHeight {
                        address: user.clone(),
                        height: None,
                    },
                )?;
                ensure!(
                    voting_power.power >= *min_voting_power,
                    ContractError::NotGatingCondition {
                        condition: self.clone(),
                        user
                    }
                );
                Ok::<_, ContractError>(())
            }
            crate::state::AdvantageOptions::Cw20(needed_amount) => {
                // We verify the sender has enough coins in their wallet
                let user_balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                    &needed_amount.address,
                    &cw20_base::msg::QueryMsg::Balance {
                        address: user.clone(),
                    },
                )?;

                ensure!(
                    user_balance.balance >= needed_amount.amount,
                    ContractError::NotGatingCondition {
                        condition: self.clone(),
                        user
                    }
                );
                Ok(())
            }
            crate::state::AdvantageOptions::Staking { min_voting_power } => {
                let stake_response: Uint128 = deps
                    .querier
                    .query_all_delegations(&user)?
                    .into_iter()
                    .map(|delegation| delegation.amount.amount)
                    .sum();

                ensure!(
                    stake_response >= *min_voting_power,
                    ContractError::NotGatingCondition {
                        condition: self.clone(),
                        user
                    }
                );
                Ok(())
            }
        }
    }
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
    pub whitelist: Option<Vec<String>>,

    pub gating_raffle: Vec<AdvantageOptionsMsg>,
}

#[cw_serde]
pub enum AdvantageOptionsMsg {
    Cw721Coin {
        nft_count: u32,
        nft_address: String,
    },
    Cw20(Cw20Coin),
    Coin(Coin),
    Sg721Token {
        nft_count: u32,
        nft_address: String,
    },
    DaoVotingPower {
        dao_address: String,
        min_voting_power: Uint128,
    },
    Staking {
        min_voting_power: Uint128,
    },
}

impl AdvantageOptionsMsg {
    pub fn check(self, api: &dyn Api) -> StdResult<AdvantageOptions> {
        Ok(match self {
            AdvantageOptionsMsg::Cw721Coin {
                nft_count,
                nft_address,
            } => {
                ensure!(
                    nft_count <= NFT_TOKEN_LIMIT,
                    StdError::generic_err("Too many nft count in nft advantage")
                );
                AdvantageOptions::Cw721Coin {
                    nft_count,
                    nft_address: api.addr_validate(&nft_address)?,
                }
            }
            AdvantageOptionsMsg::Coin(coin) => AdvantageOptions::Coin(coin),
            AdvantageOptionsMsg::Sg721Token {
                nft_count,
                nft_address,
            } => {
                ensure!(
                    nft_count <= NFT_TOKEN_LIMIT,
                    StdError::generic_err("Too many nft count in nft advantage")
                );
                AdvantageOptions::Sg721Token {
                    nft_address: api.addr_validate(&nft_address)?,
                    nft_count,
                }
            }
            AdvantageOptionsMsg::DaoVotingPower {
                dao_address,
                min_voting_power,
            } => AdvantageOptions::DaoVotingPower {
                dao_address: api.addr_validate(&dao_address)?,
                min_voting_power,
            },
            AdvantageOptionsMsg::Cw20(c) => AdvantageOptions::Cw20(Cw20CoinVerified {
                address: api.addr_validate(&c.address)?,
                amount: c.amount,
            }),
            AdvantageOptionsMsg::Staking { min_voting_power } => {
                AdvantageOptions::Staking { min_voting_power }
            }
        })
    }
}

impl From<AdvantageOptions> for AdvantageOptionsMsg {
    fn from(options: AdvantageOptions) -> AdvantageOptionsMsg {
        match options {
            AdvantageOptions::Cw721Coin {
                nft_count,
                nft_address,
            } => AdvantageOptionsMsg::Cw721Coin {
                nft_count,
                nft_address: nft_address.to_string(),
            },
            AdvantageOptions::Coin(coin) => AdvantageOptionsMsg::Coin(coin),
            AdvantageOptions::Sg721Token {
                nft_count,
                nft_address,
            } => AdvantageOptionsMsg::Sg721Token {
                nft_count,
                nft_address: nft_address.to_string(),
            },
            AdvantageOptions::DaoVotingPower {
                dao_address,
                min_voting_power,
            } => AdvantageOptionsMsg::DaoVotingPower {
                dao_address: dao_address.to_string(),
                min_voting_power,
            },
            AdvantageOptions::Cw20(c) => AdvantageOptionsMsg::Cw20(Cw20Coin {
                address: c.to_string(),
                amount: c.amount,
            }),
            AdvantageOptions::Staking { min_voting_power } => {
                AdvantageOptionsMsg::Staking { min_voting_power }
            }
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
                    Some(min_ticket_number.max(assets_len as u32))
                } else {
                    Some(assets_len as u32)
                }
            } else {
                raffle_options.min_ticket_number
            },
            whitelist: raffle_options
                .whitelist
                .map(|v| {
                    v.into_iter()
                        .map(|a| api.addr_validate(&a))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?,

            gating_raffle: raffle_options
                .gating_raffle
                .into_iter()
                .map(|options| options.check(api))
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
            whitelist: raffle_options
                .whitelist
                .map(|v| {
                    v.into_iter()
                        .map(|a| api.addr_validate(&a))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?,

            gating_raffle: raffle_options
                .gating_raffle
                .into_iter()
                .map(|options| options.check(api))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}
