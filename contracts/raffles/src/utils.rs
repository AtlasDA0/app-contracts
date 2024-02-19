#[cfg(feature = "sg")]
use sg721::ExecuteMsg as Sg721ExecuteMsg;

use crate::{
    error::ContractError,
    state::{get_raffle_state, RaffleInfo, RaffleState, CONFIG, RAFFLE_INFO, RAFFLE_TICKETS},
};
use cosmwasm_std::{
    coins, to_json_binary, Addr, BankMsg, Coin, Deps, Empty, Env, HexBinary, StdError, StdResult,
    Storage, Uint128, WasmMsg,
};
use cw721::Cw721ExecuteMsg;
use cw721_base::Extension;

use nois::{int_in_range, ProxyExecuteMsg};

use utils::{
    state::{into_cosmos_msg, AssetInfo},
    types::{CosmosMsg, Response},
};

pub fn get_nois_randomness(deps: Deps, raffle_id: u64) -> Result<Response, ContractError> {
    let raffle_info = RAFFLE_INFO.load(deps.storage, raffle_id.clone())?;
    let config = CONFIG.load(deps.storage)?;
    let id: String = raffle_id.to_string();
    let nois_fee: Coin = config.nois_proxy_coin;

    // cannot provide new randomness once value is provided
    if raffle_info.randomness.is_some() {
        return Err(ContractError::RandomnessAlreadyProvided {});
    }

    // request randomness
    let response = Response::new().add_message(WasmMsg::Execute {
        contract_addr: config.nois_proxy_addr.into_string(),
        // GetNextRandomness requests the randomness from the proxy
        // The job id is needed to know what randomness we are referring to upon reception in the callback.
        msg: to_json_binary(&ProxyExecuteMsg::GetNextRandomness {
            job_id: "raffle-".to_string() + id.as_str(),
        })?,

        funds: vec![nois_fee], // Pay from the contract
    });
    Ok(response)
}

/// Util to get the organizers and helpers messages to return when claiming a Raffle (returns the funds)
pub fn get_raffle_owner_finished_messages(
    storage: &dyn Storage,
    _env: Env,
    raffle_info: RaffleInfo,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let config = CONFIG.load(storage)?;

    // We start by splitting the fees between owner & treasury
    let total_paid = match raffle_info.raffle_ticket_price.clone() {
        // only native coins accepted for raffle fees currently
        AssetInfo::Coin(coin) => coin.amount,
        _ => return Err(ContractError::WrongFundsType {}),
    } * Uint128::from(raffle_info.number_of_tickets);

    // use raffle_fee % to calculate treasury distribution
    let treasury_amount = total_paid * config.raffle_fee;
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
                        to_address: config.owner.to_string(),
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

/// Picking the winner of the raffle
pub fn get_raffle_winner(
    deps: Deps,
    env: Env,
    raffle_id: u64,
    raffle_info: RaffleInfo,
) -> Result<Addr, ContractError> {
    // if randomness not has been provided then we expect an error
    if raffle_info.randomness.is_none() {
        return Err(ContractError::WrongStateForClaim {
            status: get_raffle_state(env, raffle_info),
        });
    }

    // We initiate the random number generator
    let randomness: [u8; 32] = HexBinary::to_array(&raffle_info.randomness.unwrap())?;

    // We pick a winner id
    let winner_id = int_in_range(randomness.into(), 0, raffle_info.number_of_tickets);
    let winner = RAFFLE_TICKETS.load(deps.storage, (raffle_id, winner_id))?;

    Ok(winner)
}

/// Util to get the raffle creator messages to return when the Raffle is cancelled (returns the raffled asset)
pub fn get_raffle_owner_messages(env: Env, raffle_info: RaffleInfo) -> StdResult<Vec<CosmosMsg>> {
    let owner: Addr = raffle_info.owner.clone();
    _get_raffle_end_asset_messages(env, raffle_info, owner.to_string())
}

/// Util to get the assets back from a raffle
fn _get_raffle_end_asset_messages(
    _env: Env,
    raffle_info: RaffleInfo,
    receiver: String,
) -> StdResult<Vec<CosmosMsg>> {
    raffle_info
        .assets
        .iter()
        .map(|asset| match asset {
            AssetInfo::Cw721Coin(nft) => {
                let message = Cw721ExecuteMsg::TransferNft {
                    recipient: receiver.clone(),
                    token_id: nft.token_id.clone(),
                };
                into_cosmos_msg(message, nft.address.clone(), None)
            }
            #[cfg(feature = "sg")]
            AssetInfo::Sg721Token(sg721_token) => {
                let message = Sg721ExecuteMsg::<Extension, Empty>::TransferNft {
                    recipient: receiver.clone(),
                    token_id: sg721_token.token_id.clone(),
                };
                into_cosmos_msg(message, sg721_token.address.clone(), None)
            }
            _ => return Err(StdError::generic_err("unreachable")),
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
    if get_raffle_state(env, raffle_info) == RaffleState::Started {
        Ok(())
    } else {
        return Err(ContractError::CantBuyTickets {});
    }
}

pub fn get_raffle_winner_messages(
    deps: Deps,
    env: Env,
    raffle_info: RaffleInfo,
    raffle_id: u64,
) -> StdResult<Vec<CosmosMsg>> {
    if raffle_info.winner.clone() == None {
        // refetch raffle winner with randomness
        get_raffle_winner(deps, env.clone(), raffle_id, raffle_info.clone()).unwrap();
    }
    let winner = raffle_info.winner.clone().unwrap();
    // generate state modifications for
    _get_raffle_end_asset_messages(env, raffle_info, winner.to_string())
}

// RAFFLE WINNER
