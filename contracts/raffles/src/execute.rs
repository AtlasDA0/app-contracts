use cosmwasm_std::{
    ensure_eq, from_json, Addr, BankMsg, Decimal, DepsMut, Empty, Env, MessageInfo, StdError,
    StdResult, Uint128,
};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use cw721_base::Extension;
use nois::NoisCallback;
use sg721::ExecuteMsg as Sg721ExecuteMsg;
use sg_std::{CosmosMsg, StargazeMsgWrapper};
use utils::state::{into_cosmos_msg, AssetInfo, Cw721Coin, Sg721Token};

use crate::{
    error::ContractError,
    msg::ExecuteMsg,
    query::{is_nft_owner, is_sg721_owner},
    state::{
        get_raffle_state, Config, RaffleInfo, RaffleOptions, RaffleOptionsMsg, RaffleState, CONFIG,
        MINIMUM_RAFFLE_DURATION, MINIMUM_RAFFLE_TIMEOUT, RAFFLE_INFO, RAFFLE_TICKETS, USER_TICKETS,
    },
    utils::{
        can_buy_ticket, get_nois_randomness, get_raffle_owner_finished_messages,
        get_raffle_owner_messages, get_raffle_winner, get_raffle_winner_messages, is_raffle_owner,
        ticket_cost,
    },
};

pub type Response = cosmwasm_std::Response<StargazeMsgWrapper>;
pub type SubMsg = cosmwasm_std::SubMsg<StargazeMsgWrapper>;

pub fn execute_create_raffle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Option<String>,
    all_assets: Vec<AssetInfo>,
    raffle_ticket_price: AssetInfo,
    raffle_options: RaffleOptionsMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.lock {
        return Err(ContractError::ContractIsLocked {});
    }

    // TODO: ensure static creation_fee has been provided
    if info.funds.len() > 1 {
        return Err(ContractError::InvalidAmount(
            "too many tokens in fee payment".to_string(),
        ));
    }

    let fee = info
        .funds
        .iter()
        .find(|c| config.creation_fee_denom.contains(&c.denom))
        .map(|c| Uint128::from(c.amount))
        .unwrap_or_else(|| Uint128::zero());

    if fee < config.creation_fee_amount {
        return Err(ContractError::InvalidRaffleFee {});
    }

    let transfer_fee: CosmosMsg = BankMsg::Send {
        to_address: config.fee_addr.to_string(),
        amount: info.funds,
    }
    .into();

    // make sure an asset was provided.
    if all_assets.is_empty() {
        return Err(ContractError::NoAssets {});
    }

    // Then we physcially transfer all the assets
    let transfer_messages: Vec<CosmosMsg> = all_assets
        .iter()
        .map(|asset| match &asset {
            AssetInfo::Cw721Coin(token) => {
                // Before the transfer, verify current NFT owner
                // Otherwise, this would cause anyone to be able to create loans in the name of the owner if a bad approval was done
                is_nft_owner(
                    deps.as_ref(),
                    info.sender.clone(),
                    token.address.to_string(),
                    token.token_id.to_string(),
                )?;

                let message = Cw721ExecuteMsg::TransferNft {
                    recipient: env.contract.address.clone().into(),
                    token_id: token.token_id.clone(),
                };

                into_cosmos_msg(message, token.address.clone(), None)
            }
            AssetInfo::Sg721Token(token) => {
                is_sg721_owner(
                    deps.as_ref(),
                    info.sender.clone(),
                    token.address.to_string(),
                    token.token_id.to_string(),
                )?;

                let message = Sg721ExecuteMsg::<Extension, Empty>::TransferNft {
                    recipient: env.contract.address.clone().into(),
                    token_id: token.token_id.clone(),
                };

                into_cosmos_msg(message, token.address.clone(), None)
            }
            _ => Err(StdError::generic_err(
                "Error generating transfer_messages: Vec<CosmosMsg>",
            )),
        })
        .collect::<Result<Vec<CosmosMsg>, StdError>>()?;
    // Then we create the internal raffle structure
    let owner = owner.map(|x| deps.api.addr_validate(&x)).transpose()?;
    let raffle_id = _create_raffle(
        deps,
        env,
        owner.clone().unwrap_or_else(|| info.sender.clone()),
        all_assets,
        raffle_ticket_price,
        raffle_options,
    )?;

    Ok(Response::new()
        .add_message(transfer_fee)
        .add_messages(transfer_messages)
        .add_attribute("action", "create_raffle")
        .add_attribute("raffle_id", raffle_id.to_string())
        .add_attribute("owner", owner.unwrap_or_else(|| info.sender.clone())))
}

pub fn _create_raffle(
    deps: DepsMut,
    env: Env,
    owner: Addr,
    all_assets: Vec<AssetInfo>,
    raffle_ticket_price: AssetInfo,
    raffle_options: RaffleOptionsMsg,
) -> Result<u64, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // We start by creating a new trade_id (simply incremented from the last id)
    let raffle_id: u64 = CONFIG
        .update(deps.storage, |mut c| -> StdResult<_> {
            c.last_raffle_id = c.last_raffle_id.map_or(Some(0), |id| Some(id + 1));
            Ok(c)
        })?
        .last_raffle_id
        .unwrap(); // This is safe because of the function architecture just there

    RAFFLE_INFO.update(deps.storage, raffle_id, |trade| match trade {
        // If the trade id already exists, the contract is faulty
        // Or an external error happened, or whatever...
        // In that case, we emit an error
        // The priority is : We do not want to overwrite existing data
        Some(_) => Err(ContractError::ExistsInRaffleInfo {}),
        None => Ok(RaffleInfo {
            owner,
            assets: all_assets.clone(),
            raffle_ticket_price: raffle_ticket_price.clone(), // No checks for the assetInfo type, the worst thing that can happen is an error when trying to buy a raffle ticket
            number_of_tickets: 0u32,
            randomness: None,
            winner: None,
            is_cancelled: false,
            raffle_options: RaffleOptions::new(env, all_assets.len(), raffle_options, config),
        }),
    })?;
    Ok(raffle_id)
}

/// Cancels a raffle
/// This function is only accessible if no raffle ticket was bought on the raffle
pub fn execute_cancel_raffle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    raffle_id: u64,
) -> Result<Response, ContractError> {
    let mut raffle_info = is_raffle_owner(deps.storage, raffle_id, info.sender)?;

    // The raffle can only be cancelled if it wasn't previously cancelled and it isn't finished
    let raffle_state = get_raffle_state(env.clone(), raffle_info.clone());

    if raffle_state != RaffleState::Created
        && raffle_state != RaffleState::Started
        && raffle_state != RaffleState::Closed
        && raffle_state != RaffleState::Finished
    {
        return Err(ContractError::WrongStateForCancel {
            status: raffle_state,
        });
    }

    // We then verify there are not tickets bought
    if raffle_info.number_of_tickets != 0 {
        return Err(ContractError::RaffleAlreadyStarted {});
    }

    // Then notify the raffle is ended
    raffle_info.is_cancelled = true;
    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    // Then we transfer the assets back to the owner
    let transfer_messages = get_raffle_owner_messages(env, raffle_info)?;
    Ok(Response::new()
        .add_messages(transfer_messages)
        .add_attribute("action", "cancel_raffle")
        .add_attribute("raffle_id", raffle_id.to_string()))
}

/// Modify the raffle characteristics
/// A parameter is only modified if it is specified in the called message
/// If None is provided, nothing changes for the parameter
/// This function is only accessible if no raffle ticket was bought on the raffle
pub fn execute_modify_raffle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    raffle_id: u64,
    raffle_ticket_price: Option<AssetInfo>,
    raffle_options: RaffleOptionsMsg,
) -> Result<Response, ContractError> {
    let mut raffle_info = is_raffle_owner(deps.storage, raffle_id, info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    // We then verify there are not tickets bought
    if raffle_info.number_of_tickets != 0 {
        return Err(ContractError::RaffleAlreadyStarted {});
    }

    // Then modify the raffle characteristics
    raffle_info.raffle_options = RaffleOptions::new_from(
        raffle_info.raffle_options,
        raffle_info.assets.len(),
        raffle_options,
        config,
    );
    // Then modify the ticket price
    if let Some(raffle_ticket_price) = raffle_ticket_price {
        raffle_info.raffle_ticket_price = raffle_ticket_price;
    }
    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    Ok(Response::new()
        .add_attribute("action", "modify_raffle")
        .add_attribute("raffle_id", raffle_id.to_string()))
}

/// Buy a ticket for a specific raffle.
///
/// `raffle_id`: The id of the raffle you want to buy a ticket to/
///
/// `assets` : the assets you want to deposit against a raffle ticket.
/// These assets must be a native coin
/// These must correspond to the raffle_info.raffle_ticket_price exactly
/// This function needs the sender to approve token transfer (for CW20 tokens) priori to the transaction
/// The next function provides a receiver message implementation if you prefer
pub fn execute_buy_tickets(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    raffle_id: u64,
    ticket_count: u32,
    assets: AssetInfo,
) -> Result<Response, ContractError> {
    // First we physcially transfer the AssetInfo
    let transfer_messages = match &assets {
        AssetInfo::Cw721Coin(token) => {
            let message = Cw721ExecuteMsg::TransferNft {
                recipient: env.contract.address.clone().into(),
                token_id: token.token_id.clone(),
            };
            vec![into_cosmos_msg(message, token.address.clone(), None)?]
        }
        AssetInfo::Sg721Token(token) => {
            let message = Sg721ExecuteMsg::<Extension, Empty>::TransferNft {
                recipient: env.contract.address.clone().into(),
                token_id: token.token_id.clone(),
            };
            vec![into_cosmos_msg(message, token.address.clone(), None)?]
        }
        // or verify the sent coins match the message coins
        AssetInfo::Coin(coin) => {
            if coin.amount != Uint128::zero()
                && (info.funds.len() != 1
                    || info.funds[0].denom != coin.denom
                    || info.funds[0].amount != coin.amount)
            {
                return Err(ContractError::AssetMismatch {});
            }
            vec![]
        }
        _ => return Err(ContractError::WrongAssetType {}),
    };

    // Then we verify the funds sent match the raffle conditions and we save the ticket that was bought
    _buy_tickets(
        deps,
        env.clone(),
        info.sender.clone(),
        raffle_id,
        ticket_count,
        assets,
    )?;

    Ok(Response::new()
        .add_messages(transfer_messages)
        .add_attribute("action", "buy_ticket")
        .add_attribute("raffle_id", raffle_id.to_string())
        .add_attribute("purchaser", info.sender)
        .add_attribute("ticket_count", ticket_count.to_string())
        .add_attribute("timestamp", env.block.time.to_string()))
}

/// Creates new raffle tickets and assigns them to the sender
/// Internal function that doesn't check anything and buys multiple tickets
/// The arguments are described on the execute_buy_tickets function above.
pub fn _buy_tickets(
    deps: DepsMut,
    env: Env,
    owner: Addr,
    raffle_id: u64,
    ticket_count: u32,
    assets: AssetInfo,
) -> Result<(), ContractError> {
    let mut raffle_info = RAFFLE_INFO.load(deps.storage, raffle_id)?;

    // We first check the sent assets match the raffle assets
    if ticket_cost(raffle_info.clone(), ticket_count)? != assets {
        return Err(ContractError::PaymentNotSufficient {
            assets_wanted: raffle_info.raffle_ticket_price,
            assets_received: assets,
        });
    }

    // We then check the raffle is in the right state
    can_buy_ticket(env, raffle_info.clone())?;

    // Then we check the user has the right to buy `ticket_count` more tickets
    if let Some(max_ticket_per_address) = raffle_info.raffle_options.max_ticket_per_address {
        let current_ticket_count = USER_TICKETS
            .load(deps.storage, (&owner, raffle_id))
            .unwrap_or(0);
        if current_ticket_count + ticket_count > max_ticket_per_address {
            return Err(ContractError::TooMuchTicketsForUser {
                max: max_ticket_per_address,
                nb_before: current_ticket_count,
                nb_after: current_ticket_count + ticket_count,
            });
        }
    }

    // Then we check there are some ticket left to buy
    if let Some(max_participant_number) = raffle_info.raffle_options.max_participant_number {
        if raffle_info.number_of_tickets + ticket_count > max_participant_number {
            return Err(ContractError::TooMuchTickets {
                max: max_participant_number,
                nb_before: raffle_info.number_of_tickets,
                nb_after: raffle_info.number_of_tickets + ticket_count,
            });
        }
    };

    // Then we save the sender to the bought tickets
    for n in 0..ticket_count {
        RAFFLE_TICKETS.save(
            deps.storage,
            (raffle_id, raffle_info.number_of_tickets + n),
            &owner,
        )?;
    }

    USER_TICKETS.update::<_, ContractError>(deps.storage, (&owner, raffle_id), |x| match x {
        Some(current_ticket_count) => Ok(current_ticket_count + ticket_count),
        None => Ok(ticket_count),
    })?;
    raffle_info.number_of_tickets += ticket_count;

    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    Ok(())
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    wrapper: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(&wrapper.sender)?;
    match from_json(&wrapper.msg)? {
        ExecuteMsg::BuyTicket {
            raffle_id,
            ticket_count,
            sent_assets,
        } => {
            // First we make sure the received Asset is the one specified in the message
            match sent_assets.clone() {
                AssetInfo::Cw721Coin(Cw721Coin {
                    address: _address,
                    token_id,
                }) => {
                    if token_id == wrapper.token_id {
                        // The asset is a match, we can create the raffle object and return
                        _buy_tickets(
                            deps,
                            env,
                            sender.clone(),
                            raffle_id,
                            ticket_count,
                            sent_assets,
                        )?;

                        Ok(Response::new()
                            .add_attribute("action", "buy_ticket")
                            .add_attribute("raffle_id", raffle_id.to_string())
                            .add_attribute("owner", sender))
                    } else {
                        Err(ContractError::AssetMismatch {})
                    }
                }
                AssetInfo::Sg721Token(Sg721Token {
                    address: _address,
                    token_id,
                }) => {
                    if token_id == wrapper.token_id {
                        // The asset is a match, we can create the raffle object and return
                        _buy_tickets(
                            deps,
                            env,
                            sender.clone(),
                            raffle_id,
                            ticket_count,
                            sent_assets,
                        )?;

                        Ok(Response::new()
                            .add_attribute("action", "buy_ticket")
                            .add_attribute("raffle_id", raffle_id.to_string())
                            .add_attribute("owner", sender))
                    } else {
                        Err(ContractError::AssetMismatch {})
                    }
                }
                _ => Err(ContractError::AssetMismatch {}),
            }
        }
        _ => Err(ContractError::Unauthorized {}),
    }
}

pub fn execute_receive_nois(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    callback: NoisCallback,
    raffle_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut raffle_info = RAFFLE_INFO.load(deps.storage, raffle_id)?;

    // callback should only be allowed to be called by the proxy contract
    // otherwise anyone can cut the randomness workflow and cheat the randomness by sending the randomness directly to this contract
    ensure_eq!(
        info.sender,
        config.nois_proxy_addr,
        ContractError::UnauthorizedReceive
    );
    let randomness: [u8; 32] = callback
        .randomness
        .to_array()
        .map_err(|_| ContractError::InvalidRandomness)?;

    // We make sure the raffle has not updated the global randomness yet
    if raffle_info.randomness != None {
        return Err(ContractError::RandomnessAlreadyProvided {});
    } else {
        raffle_info.randomness = Some(randomness);
    };

    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    Ok(Response::default())
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    raffle_id: u64,
) -> Result<Response, ContractError> {
    // Loading the raffle object
    let mut raffle_info = RAFFLE_INFO.load(deps.storage, raffle_id)?;

    // We make sure the raffle is ended, and randomness from nois has been provided.
    let raffle_state = get_raffle_state(env.clone(), raffle_info.clone());

    if raffle_state != RaffleState::Finished {
        return Err(ContractError::WrongStateForClaim {
            status: raffle_state,
        });
    }

    // If there was no participant, the winner is the raffle owner and we pay no fees whatsoever
    if raffle_info.number_of_tickets == 0u32 {
        raffle_info.winner = Some(raffle_info.owner.clone());
    } else {
        // We get the winner of the raffle and save it to the contract. The raffle is now claimed !
        let winner = get_raffle_winner(deps.as_ref(), env.clone(), raffle_id, raffle_info.clone())?;
        raffle_info.winner = Some(winner);
    }
    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    // We send the assets to the winner
    let winner_transfer_messages = get_raffle_winner_messages(env.clone(), raffle_info.clone())?;
    let funds_transfer_messages =
        get_raffle_owner_finished_messages(deps.storage, env, raffle_info.clone())?;
    // We distribute the ticket prices to the owner and in part to the treasury
    Ok(Response::new()
        .add_messages(winner_transfer_messages)
        .add_messages(funds_transfer_messages)
        .add_attribute("action", "claim")
        .add_attribute("raffle_id", raffle_id.to_string())
        .add_attribute("winner", raffle_info.winner.unwrap()))
}

/// Update the randomness assigned to a raffle
/// This allows trustless and un-predictable randomness to the raffle contract.
/// The randomness providers will get a small cut of the raffle tickets (to reimburse the tx fees and incentivize adding randomness)
pub fn execute_update_randomness(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    raffle_id: u64,
) -> Result<Response, ContractError> {
    // We check the raffle can receive randomness (good state)
    let raffle_info = RAFFLE_INFO.load(deps.storage, raffle_id)?;
    let raffle_state = get_raffle_state(env, raffle_info);
    if raffle_state != RaffleState::Closed {
        return Err(ContractError::WrongStateForRandmness {
            status: raffle_state,
        });
    }
    // We assert the randomness is correct
    get_nois_randomness(deps.as_ref(), raffle_id)
    // get randomness from nois.network
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _name: Option<String>,
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
) -> Result<Response, ContractError> {
    //TODO: let mut config
    let config = CONFIG.load(deps.storage)?;
    // ensure msg sender is admin
    ensure_eq!(info.sender, config.owner, ContractError::Unauthorized);
    // TODO: check if new value is_valid_name
    let name = config.name;
    let owner = match owner {
        Some(ow) => deps.api.addr_validate(&ow)?,
        None => config.owner,
    };
    let fee_addr = match fee_addr {
        Some(fea) => deps.api.addr_validate(&fea)?,
        None => config.fee_addr,
    };
    let minimum_raffle_duration = match minimum_raffle_duration {
        Some(mrd) => mrd.max(MINIMUM_RAFFLE_DURATION),
        None => config.minimum_raffle_duration,
    };
    let minimum_raffle_timeout = match minimum_raffle_timeout {
        Some(mrt) => mrt.max(MINIMUM_RAFFLE_TIMEOUT),
        None => config.minimum_raffle_timeout,
    };
    let raffle_fee = match raffle_fee {
        Some(rf) => rf,
        None => config.raffle_fee,
    };
    // let rand_fee = match rand_fee {
    //     Some(raf) => raf,
    //     None => config.rand_fee,
    // };
    let nois_proxy_addr = match nois_proxy_addr {
        Some(prx) => deps.api.addr_validate(&prx)?,
        None => config.nois_proxy_addr,
    };
    let nois_proxy_denom = match nois_proxy_denom {
        Some(npr) => npr,
        None => config.nois_proxy_denom,
    };
    let nois_proxy_amount = match nois_proxy_amount {
        Some(npa) => npa,
        None => config.nois_proxy_amount,
    };
    let creation_fee_denom = match creation_fee_denom {
        Some(crf) => crf,
        None => config.creation_fee_denom,
    };
    let creation_fee_amount = match creation_fee_amount {
        Some(crf) => crf,
        None => config.creation_fee_amount,
    };
    // we have a seperate function to lock a raffle, so we skip here
    let lock = config.lock;
    // we do not want to be able to manually update the last raffle id.
    let last_raffle_id = config.last_raffle_id;

    CONFIG.save(
        deps.storage,
        &Config {
            name,
            owner,
            fee_addr,
            last_raffle_id,
            minimum_raffle_duration,
            minimum_raffle_timeout,
            creation_fee_amount,
            creation_fee_denom,
            raffle_fee,
            lock,
            nois_proxy_addr,
            nois_proxy_denom,
            nois_proxy_amount,
        },
    )?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Locking the contract (lock=true) means preventing the creation of new raffles
/// Tickets can still be bought and NFTs retrieved when a contract is locked
pub fn execute_toggle_lock(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    lock: bool,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    // check the calling address is the authorised multisig
    ensure_eq!(info.sender, config.owner, ContractError::Unauthorized);

    config.lock = lock;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("parameter", "contract_lock")
        .add_attribute("value", lock.to_string()))
}
