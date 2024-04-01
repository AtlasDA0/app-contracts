use std::collections::HashMap;

use dao_interface::voting::VotingPowerAtHeightResponse;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro256PlusPlus};
#[cfg(feature = "sg")]
use sg721::ExecuteMsg as Sg721ExecuteMsg;

use crate::{
    error::ContractError,
    state::{get_raffle_state, RaffleInfo, RaffleState, CONFIG, RAFFLE_INFO, RAFFLE_TICKETS},
};
use cosmwasm_std::{
    coins, ensure, to_json_binary, Addr, BankMsg, Coin, Deps, Empty, Env, HexBinary, Order,
    StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw721::Cw721ExecuteMsg;
use cw721_base::Extension;

use nois::ProxyExecuteMsg;
use rand::Rng;
use utils::{
    state::{into_cosmos_msg, AssetInfo},
    types::CosmosMsg,
};

pub fn get_nois_randomness(deps: Deps, raffle_id: u64) -> Result<CosmosMsg, ContractError> {
    let raffle_info = RAFFLE_INFO.load(deps.storage, raffle_id)?;
    let config = CONFIG.load(deps.storage)?;
    let id: String = raffle_id.to_string();
    let nois_fee: Coin = config.nois_proxy_coin;

    // cannot provide new randomness once value is provided
    if raffle_info.randomness.is_some() {
        return Err(ContractError::RandomnessAlreadyProvided {});
    }

    // request randomness
    Ok(WasmMsg::Execute {
        contract_addr: config.nois_proxy_addr.into_string(),
        // GetNextRandomness requests the randomness from the proxy
        // The job id is needed to know what randomness we are referring to upon reception in the callback.
        msg: to_json_binary(&ProxyExecuteMsg::GetNextRandomness {
            job_id: "raffle-".to_string() + id.as_str(),
        })?,

        funds: vec![nois_fee], // Pay from the contract
    }
    .into())
}

/// Util to get the organizers and helpers messages to return when claiming a Raffle (returns the funds)
pub fn get_raffle_owner_funds_finished_messages(
    deps: Deps,
    _env: Env,
    raffle_info: RaffleInfo,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // We start by splitting the fees between owner & treasury
    let total_paid = match raffle_info.raffle_ticket_price.clone() {
        // only native coins accepted for raffle fees currently
        AssetInfo::Coin(coin) => coin.amount,
        _ => return Err(ContractError::WrongFundsType {}),
    } * Uint128::from(raffle_info.number_of_tickets);

    // use raffle_fee % to calculate treasury distribution
    let mut treasury_amount = total_paid * config.raffle_fee;
    {
        if let Some(nft_address) = config.atlas_dao_nft_address {
            // If the owner is an Atlas DAO NFT holder, they don't get the treasury_fee
            let token_hold: cw721::TokensResponse = deps.querier.query_wasm_smart(
                nft_address,
                &sg721_base::msg::QueryMsg::Tokens {
                    owner: raffle_info.owner.to_string(),
                    start_after: None,
                    limit: Some(1),
                },
            )?;
            if !token_hold.tokens.is_empty() {
                treasury_amount = Uint128::zero();
            }
        }
    }

    {
        // If the owner is a Stargaze staker, they get a discount on the treasury fee
        let stake_response: Uint128 = deps
            .querier
            .query_all_delegations(&raffle_info.owner)?
            .into_iter()
            .map(|delegation| delegation.amount.amount)
            .sum();

        println!("{:?} - {:?}", stake_response, config.staker_fee_discount);

        if stake_response > config.staker_fee_discount.minimum_amount {
            treasury_amount -= treasury_amount * config.staker_fee_discount.discount;
        }
    }

    let owner_amount = total_paid - treasury_amount;

    // Then we craft the messages needed for asset transfers
    match raffle_info.raffle_ticket_price {
        AssetInfo::Coin(coin) => {
            let mut messages: Vec<CosmosMsg> = vec![];
            if treasury_amount != Uint128::zero() {
                messages.push(
                    BankMsg::Send {
                        to_address: config.fee_addr.to_string(),
                        amount: coins(treasury_amount.u128(), coin.denom.clone()),
                    }
                    .into(),
                );
            };
            if owner_amount != Uint128::zero() {
                messages.push(
                    BankMsg::Send {
                        to_address: raffle_info.owner.to_string(),
                        amount: coins(owner_amount.u128(), coin.denom),
                    }
                    .into(),
                );
            };

            Ok(messages)
        }
        _ => Err(ContractError::WrongFundsType {}),
    }
}

/// Util to get the refund of funds for raffle participants
pub fn get_raffle_refund_funds_finished_messages(
    storage: &dyn Storage,
    _env: Env,
    raffle_info: RaffleInfo,
    raffle_id: u64,
) -> Result<Vec<CosmosMsg>, ContractError> {
    // We refund all the raffle ticket funds to the tickets buyers
    let raffle_ticket_buyers = RAFFLE_TICKETS
        .prefix(raffle_id)
        .range(storage, None, None, Order::Descending)
        .map(|r| {
            r.and_then(|(_k, v)| {
                // We get the funds transfer message
                match &raffle_info.raffle_ticket_price {
                    AssetInfo::Coin(ticket_price) => Ok(BankMsg::Send {
                        to_address: v.to_string(),
                        amount: vec![ticket_price.clone()],
                    }
                    .into()),
                    _ => Err(StdError::generic_err("Invalid Ticket")),
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(raffle_ticket_buyers)
}

/// Picking the winner of the raffle
pub fn get_raffle_winners(
    deps: Deps,
    env: Env,
    raffle_id: u64,
    raffle_info: RaffleInfo,
) -> Result<Vec<Addr>, ContractError> {
    // if randomness not has been provided then we expect an error
    if raffle_info.randomness.is_none() {
        return Err(ContractError::WrongStateForClaim {
            status: get_raffle_state(env, &raffle_info),
        });
    }

    // We initiate the random number generator
    let randomness: [u8; 32] = HexBinary::to_array(&raffle_info.randomness.unwrap())?;

    let nb_winners = if raffle_info.raffle_options.one_winner_per_asset {
        raffle_info.assets.len()
    } else {
        1
    };

    let winner_ids =
        pick_m_single_winners_among_n(randomness, raffle_info.number_of_tickets, nb_winners)?;

    let winners = winner_ids
        .into_iter()
        .map(|winner_id| RAFFLE_TICKETS.load(deps.storage, (raffle_id, winner_id)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(winners)
}

/// In this function, we are getting nb_winners different winners among n ticket.
/// We assume that nb_winners <= n here
/// There is inspiration from nois::ints_in_range
/// Principle of the algorithm
/// At step 0, you have an array [0, n-1] of n elements
/// 1. You take a random number betwwen 0 and n-1
/// 2. You select this number as the first winner
/// 3. Then you exchange this number with the elements in nth place inside the vector
/// 4. You start again with the same vector where you remove the last elements
/// Because this has at least 0(n) complexity, here, we simulate this vector with the map:HashMap
/// This structure allows us to store the maps without having to store the whole vector
/// At step k, you have a vector of length n-k in which you want to take an element.
/// The index of this element here is picked at random (selected_index)
/// The element at this index is map[selected_index]
/// Finally, you replace this element with the current element at index n-k
/// Because element at index n-k will not be used anymore, we just need to change the value at the selected index by the value a index n-k
pub fn pick_m_single_winners_among_n(
    randomness: [u8; 32],
    n: u32,
    nb_winners: usize, // m
) -> Result<Vec<u32>, ContractError> {
    let mut map = HashMap::new();
    let mut rng = make_prng(randomness);
    let mut results = vec![];
    for m in 0..nb_winners {
        // We start by selecting a number between 0 and the current maximum
        let current_maximum = n - 1 - m as u32;
        let selected_index = rng.gen_range(0..=current_maximum);

        // We consider the array to
        let selected_element = *map.get(&selected_index).unwrap_or(&selected_index);
        map.insert(
            selected_index,
            *map.get(&current_maximum).unwrap_or(&current_maximum),
        );
        results.push(selected_element);
    }

    Ok(results)
}

pub fn make_prng(randomness: [u8; 32]) -> Xoshiro256PlusPlus {
    // A PRNG that is not cryptographically secure.
    // See https://docs.rs/rand/0.8.5/rand/rngs/struct.SmallRng.html
    // where this is used for 32 bit systems.
    // We don't use the SmallRng in order to get the same implementation
    // in unit tests (64 bit dev machines) and the real contract (32 bit Wasm)

    // We chose the 256 bit variant as it allows using the full randomness value
    // but this might be overkill in out context. Maybe the 32bit version is better suited
    // for running in the wasm32 target.
    Xoshiro256PlusPlus::from_seed(randomness)
}

/// Util to get the raffle creator messages to return when the Raffle is cancelled (returns the raffled asset)
pub fn get_raffle_owner_messages(env: Env, raffle_info: RaffleInfo) -> StdResult<Vec<CosmosMsg>> {
    let owner: Addr = raffle_info.owner.clone();
    _get_raffle_end_asset_messages(env, raffle_info, vec![owner])
}

/// Util to get the assets back from a raffle
fn _get_raffle_end_asset_messages(
    _env: Env,
    raffle_info: RaffleInfo,
    receivers: Vec<Addr>,
) -> StdResult<Vec<CosmosMsg>> {
    raffle_info
        .assets
        .iter()
        .enumerate()
        .map(|(i, asset)| {
            let receiver = if receivers.len() == 1 {
                receivers[0].to_string()
            } else {
                receivers[i].to_string()
            };
            match asset {
                AssetInfo::Cw721Coin(nft) => {
                    let message = Cw721ExecuteMsg::TransferNft {
                        recipient: receiver,
                        token_id: nft.token_id.clone(),
                    };
                    into_cosmos_msg(message, nft.address.clone(), None)
                }
                #[cfg(feature = "sg")]
                AssetInfo::Sg721Token(sg721_token) => {
                    let message = Sg721ExecuteMsg::<Extension, Empty>::TransferNft {
                        recipient: receiver,
                        token_id: sg721_token.token_id.clone(),
                    };
                    into_cosmos_msg(message, sg721_token.address.clone(), None)
                }
                _ => Err(StdError::generic_err("unreachable")),
            }
        })
        .collect()
}

pub fn is_raffle_owner(
    storage: &dyn Storage,
    raffle_id: u64,
    sender: Addr,
) -> Result<RaffleInfo, ContractError> {
    let raffle_info = RAFFLE_INFO.load(storage, raffle_id)?;
    if sender == raffle_info.owner {
        Ok(raffle_info)
    } else {
        Err(ContractError::Unauthorized {})
    }
}

/// Computes the ticket cost for multiple tickets bought together
pub fn ticket_cost(raffle_info: RaffleInfo, ticket_count: u32) -> Result<AssetInfo, ContractError> {
    // enforces only Coin is a ticket cost currently.
    Ok(match raffle_info.raffle_ticket_price {
        AssetInfo::Coin(x) => AssetInfo::Coin(Coin {
            denom: x.denom,
            amount: Uint128::from(ticket_count) * x.amount,
        }),
        _ => return Err(ContractError::WrongAssetType {}),
    })
}

/// Can only buy a ticket when the raffle has started and is not closed
pub fn can_buy_ticket(env: Env, raffle_info: RaffleInfo) -> Result<(), ContractError> {
    if get_raffle_state(env, &raffle_info) == RaffleState::Started {
        Ok(())
    } else {
        Err(ContractError::CantBuyTickets {})
    }
}

pub fn buyer_can_buy_ticket(
    deps: Deps,
    raffle_info: &RaffleInfo,
    buyer: String,
) -> Result<(), ContractError> {
    // We also check if the raffle is token gated
    raffle_info
        .raffle_options
        .gating_raffle
        .iter()
        .try_for_each(|options| match options {
            crate::state::GatingOptions::Cw721Coin(address) => {
                let owner_query: cw721::TokensResponse = deps.querier.query_wasm_smart(
                    address,
                    &cw721_base::QueryMsg::<Empty>::Tokens {
                        owner: buyer.clone(),
                        start_after: None,
                        limit: None,
                    },
                )?;
                ensure!(
                    !owner_query.tokens.is_empty(),
                    ContractError::NotGatingCondition {
                        condition: options.clone(),
                        user: buyer.clone()
                    }
                );
                Ok::<_, ContractError>(())
            }
            crate::state::GatingOptions::Coin(needed_coins) => {
                // We verify the sender has enough coins in their wallet
                let user_balance = deps
                    .querier
                    .query_balance(buyer.clone(), needed_coins.denom.clone())?;

                ensure!(
                    user_balance.amount >= needed_coins.amount,
                    ContractError::NotGatingCondition {
                        condition: options.clone(),
                        user: buyer.clone()
                    }
                );
                Ok(())
            }
            crate::state::GatingOptions::Sg721Token(address) => {
                let owner_query: cw721::TokensResponse = deps.querier.query_wasm_smart(
                    address,
                    &sg721_base::QueryMsg::Tokens {
                        owner: buyer.clone(),
                        start_after: None,
                        limit: None,
                    },
                )?;
                ensure!(
                    !owner_query.tokens.is_empty(),
                    ContractError::NotGatingCondition {
                        condition: options.clone(),
                        user: buyer.to_string()
                    }
                );
                Ok::<_, ContractError>(())
            }
            crate::state::GatingOptions::DaoVotingPower {
                dao_address,
                min_voting_power,
            } => {
                let voting_power: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
                    dao_address,
                    &dao_interface::msg::QueryMsg::VotingPowerAtHeight {
                        address: buyer.clone(),
                        height: None,
                    },
                )?;
                ensure!(
                    voting_power.power >= *min_voting_power,
                    ContractError::NotGatingCondition {
                        condition: options.clone(),
                        user: buyer.clone()
                    }
                );
                Ok::<_, ContractError>(())
            }
            crate::state::GatingOptions::Cw20(needed_amount) => {
                // We verify the sender has enough coins in their wallet
                let user_balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                    &needed_amount.address,
                    &cw20_base::msg::QueryMsg::Balance {
                        address: buyer.clone(),
                    },
                )?;

                ensure!(
                    user_balance.balance >= needed_amount.amount,
                    ContractError::NotGatingCondition {
                        condition: options.clone(),
                        user: buyer.clone()
                    }
                );
                Ok(())
            }
        })
}

pub fn get_raffle_winner_messages(
    _deps: Deps,
    env: Env,
    raffle_info: RaffleInfo,
) -> StdResult<Vec<CosmosMsg>> {
    let winners = raffle_info.winners.clone();
    // generate state modifications for
    _get_raffle_end_asset_messages(env, raffle_info, winners)
}

// RAFFLE WINNER

#[cfg(test)]
pub mod test {
    use cosmwasm_std::HexBinary;

    use crate::error::ContractError;

    use super::pick_m_single_winners_among_n;

    #[test]
    fn large_random() -> Result<(), ContractError> {
        let n = 50;
        let m = 48;

        // We execute the random function and verify we get different numbers between 0 and 49
        let mut winners = pick_m_single_winners_among_n(
            HexBinary::from_hex(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa115",
            )?
            .to_array()?,
            n,
            m,
        )?;
        // Verify all the results are between 0 and 49
        assert!(winners.iter().all(|k| *k < n));

        // Verify all are greater than the previous
        winners.sort();

        winners.iter().reduce(|prev, e| {
            if prev >= e {
                panic!("all elements should be different")
            } else {
                e
            }
        });

        Ok(())
    }
}
