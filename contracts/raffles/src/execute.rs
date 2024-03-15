use cosmwasm_std::{
    ensure, ensure_eq, from_json, to_json_binary, Addr, BankMsg, Coin, Decimal, DepsMut, Empty,
    Env, MessageInfo, StdError, StdResult, Uint128, WasmMsg,
};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use cw721_base::Extension;
use nois::{NoisCallback, ProxyExecuteMsg};

#[cfg(feature = "sg")]
use {crate::query::is_sg721_owner, sg721::ExecuteMsg as Sg721ExecuteMsg};

use utils::{
    state::{into_cosmos_msg, is_valid_comment, is_valid_name, AssetInfo},
    types::{CosmosMsg, Response},
};

use crate::{
    error::ContractError,
    msg::ExecuteMsg,
    query::is_nft_owner,
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

#[allow(clippy::too_many_arguments)]
pub fn execute_create_raffle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Option<String>,
    all_assets: Vec<AssetInfo>,
    raffle_ticket_price: AssetInfo,
    raffle_options: RaffleOptionsMsg,
    autocycle: Option<bool>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // verify ticket cost atleast 1
    match raffle_ticket_price.clone() {
        AssetInfo::Cw721Coin(_) => return Err(ContractError::InvalidTicketCost),
        AssetInfo::Coin(coin) => {
            if coin.amount < Uint128::one() {
                return Err(ContractError::InvalidTicketCost {});
            };
        }
        AssetInfo::Sg721Token(_) => return Err(ContractError::InvalidTicketCost),
    }

    if config.locks.lock || config.locks.sudo_lock {
        return Err(ContractError::ContractIsLocked {});
    }

    // checks if the required fee was sent.
    let fee = info
        .funds
        .into_iter()
        .find(|c| config.creation_coins.contains(c))
        .unwrap_or_default();

    // prevents 0 ticket costs

    // if the fee is not equal to one of the raffle fee coins set
    // return an invalid raffle fee error
    if !config.creation_coins.contains(&fee) {
        return Err(ContractError::InvalidRaffleFee {});
    }

    // checks comment size
    if !is_valid_comment(&raffle_options.comment.clone().unwrap_or_default()) {
        return Err(ContractError::Std(StdError::generic_err(
            "Comment too long. max = (20000 UTF-8 bytes)",
        )));
    }

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
                // Transfer the nft from raffle creator to the raffle contract.
                let message = Cw721ExecuteMsg::TransferNft {
                    recipient: env.contract.address.clone().into(),
                    token_id: token.token_id.clone(),
                };

                into_cosmos_msg(message, token.address.clone(), None)
            }
            #[cfg(feature = "sg")]
            AssetInfo::Sg721Token(token) => {
                // verify ownership
                is_sg721_owner(
                    deps.as_ref(),
                    info.sender.clone(),
                    token.address.to_string(),
                    token.token_id.to_string(),
                )?;
                // Transfer message
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
    // defines the fee token to send to nois-proxy, by the smart contract
    let nois_fee: Coin = config.nois_proxy_coin;
    let raffle_id = _create_raffle(
        deps,
        env.clone(),
        owner.clone().unwrap_or_else(|| info.sender.clone()),
        all_assets,
        raffle_ticket_price,
        raffle_options.clone(),
    )?;
    let raffle_lifecycle = raffle_options
        .raffle_start_timestamp
        .unwrap_or_default()
        .plus_seconds(raffle_options.clone().raffle_duration.unwrap_or_default())
        .plus_seconds(6);

    let res = Response::new();

    // bypass sending fee if static raffle creation cost is 0
    if !fee.amount.is_zero() {
        // transfer only the calculated fee amount from the coins sent
        let transfer_fee_msg: CosmosMsg = BankMsg::Send {
            to_address: config.fee_addr.to_string(),
            amount: vec![fee],
        }
        .into();
        // add msg to response
        res.clone().add_message(transfer_fee_msg);
    };

    // GetNextRandomness requests the randomness from the proxy after the expected raffle duration
    // The job id is needed to know what randomness we are referring to upon reception in the callback.
    if autocycle == Some(true) {
        let nois_msg: WasmMsg = WasmMsg::Execute {
            contract_addr: config.nois_proxy_addr.into_string(),
            msg: to_json_binary(&ProxyExecuteMsg::GetRandomnessAfter {
                after: raffle_lifecycle,
                job_id: "raffle-".to_string() + raffle_id.to_string().as_str(),
            })?,
            funds: vec![nois_fee], // Pay from the contract
        };
        res.clone().add_message(nois_msg);
    }

    Ok(res
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
            raffle_options: RaffleOptions::new(
                deps.api,
                env,
                all_assets.len(),
                raffle_options,
                config,
            )?,
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
    env: Env,
    info: MessageInfo,
    raffle_id: u64,
    raffle_ticket_price: Option<AssetInfo>,
    raffle_options: RaffleOptionsMsg,
) -> Result<Response, ContractError> {
    let mut raffle_info = is_raffle_owner(deps.storage, raffle_id, info.sender)?;
    let raffle_state = get_raffle_state(env, raffle_info.clone());
    let config = CONFIG.load(deps.storage)?;
    // We then verify there are not tickets bought
    if raffle_info.number_of_tickets != 0 {
        return Err(ContractError::RaffleAlreadyStarted {});
    }
    if raffle_state != RaffleState::Created && raffle_state != RaffleState::Started {
        return Err(ContractError::WrongStateForModify {
            status: raffle_state,
        });
    }

    // checks comment size
    if !is_valid_comment(
        &raffle_info
            .raffle_options
            .comment
            .clone()
            .unwrap_or_default(),
    ) {
        return Err(ContractError::Std(StdError::generic_err(
            "Comment too long. max = (20000 UTF-8 bytes)",
        )));
    }

    // Then modify the raffle characteristics
    raffle_info.raffle_options = RaffleOptions::new_from(
        deps.api,
        raffle_info.raffle_options,
        raffle_info.assets.len(),
        raffle_options,
        config,
    )?;
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
    let transfer_messages: Vec<cosmwasm_std::CosmosMsg<sg_std::StargazeMsgWrapper>> = match &assets
    {
        // TODO: implement support to provide nft tokens as ticket price
        // AssetInfo::Cw721Coin(token) => {
        //     let message = Cw721ExecuteMsg::TransferNft {
        //         recipient: env.contract.address.clone().into(),
        //         token_id: token.token_id.clone(),
        //     };
        //     vec![into_cosmos_msg(message, token.address.clone(), None)?]
        // }
        // #[cfg(feature = "sg")]
        // AssetInfo::Sg721Token(token) => {
        //     let message = Sg721ExecuteMsg::<Extension, Empty>::TransferNft {
        //         recipient: env.contract.address.clone().into(),
        //         token_id: token.token_id.clone(),
        //     };
        //     vec![into_cosmos_msg(message, token.address.clone(), None)?]
        // }

        // or verify the sent coins match the message coins.
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
    let tc = ticket_cost(raffle_info.clone(), ticket_count)?;
    if let AssetInfo::Coin(x) = tc.clone() {
        if !x.amount.is_zero() && tc != assets {
            return Err(ContractError::PaymentNotSufficient {
                ticket_count,
                assets_wanted: raffle_info.raffle_ticket_price,
                // TODO: print correct assets_wanted value
                assets_received: assets,
            });
        }
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
    if let Some(max_ticket_number) = raffle_info.raffle_options.max_ticket_number {
        if raffle_info.number_of_tickets + ticket_count > max_ticket_number {
            return Err(ContractError::TooMuchTickets {
                max: max_ticket_number,
                nb_before: raffle_info.number_of_tickets,
                nb_after: raffle_info.number_of_tickets + ticket_count,
            });
        }
    };

    if let Some(max_tickets_per_raffle) = raffle_info.raffle_options.max_ticket_number {
        if raffle_info.number_of_tickets + ticket_count > max_tickets_per_raffle {
            return Err(ContractError::TooMuchTickets {
                max: max_tickets_per_raffle,
                nb_before: raffle_info.number_of_tickets,
                nb_after: raffle_info.number_of_tickets + ticket_count,
            });
        }
    }

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
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    wrapper: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    // let sender = deps.api.addr_validate(&wrapper.sender)?;
    match from_json(wrapper.msg)? {
        ExecuteMsg::BuyTicket {
            raffle_id: _,
            ticket_count: _,
            sent_assets,
        } => {
            // First we make sure the received Asset is the one specified in the message
            match sent_assets.clone() {
                // AssetInfo::Cw721Coin(Cw721Coin {
                //     address: _address,
                //     token_id,
                // }) => {
                //     if token_id == wrapper.token_id {
                //         // The asset is a match, we can create the raffle object and return
                //         _buy_tickets(
                //             deps,
                //             env,
                //             sender.clone(),
                //             raffle_id,
                //             ticket_count,
                //             sent_assets,
                //         )?;

                //         Ok(Response::new()
                //             .add_attribute("action", "buy_ticket")
                //             .add_attribute("raffle_id", raffle_id.to_string())
                //             .add_attribute("owner", sender))
                //     } else {
                //         Err(ContractError::AssetMismatch {})
                //     }
                // }
                // #[cfg(feature = "sg")]
                // AssetInfo::Sg721Token(Sg721Token {
                //     address: _address,
                //     token_id,
                // }) => {
                //     if token_id == wrapper.token_id {
                //         // The asset is a match, we can create the raffle object and return
                //         _buy_tickets(
                //             deps,
                //             env,
                //             sender.clone(),
                //             raffle_id,
                //             ticket_count,
                //             sent_assets,
                //         )?;

                //         Ok(Response::new()
                //             .add_attribute("action", "buy_ticket")
                //             .add_attribute("raffle_id", raffle_id.to_string())
                //             .add_attribute("owner", sender))
                //     } else {
                //         Err(ContractError::AssetMismatch {})
                //     }
                // }
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
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

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

    // extract participant address
    let job_id = callback.job_id;
    let raffle_id = job_id
        .strip_prefix("raffle-")
        .expect("Strange, how is the job-id not prefixed with raffle-");
    let raffle_id: u64 = raffle_id.parse().unwrap();

    let mut raffle_info = RAFFLE_INFO.load(deps.storage, raffle_id)?;

    // We make sure the raffle has not updated the global randomness yet
    if raffle_info.randomness.is_some() {
        return Err(ContractError::RandomnessAlreadyProvided {});
    } else {
        raffle_info.randomness = Some(randomness.into());
    };
    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    Ok(Response::default())
}

pub fn execute_determine_winner(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    raffle_id: u64,
) -> Result<Response, ContractError> {
    // Loads the raffle id and makes sure the raffle has ended and randomness from nois has been provided.
    let mut raffle_info = RAFFLE_INFO.load(deps.storage, raffle_id)?;
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
        // We calculate the winner of the raffle and save it to the contract. The raffle is now claimed !
        let winner = get_raffle_winner(deps.as_ref(), env.clone(), raffle_id, raffle_info.clone())?;
        raffle_info.winner = Some(winner);
    }
    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    // We send the assets to the winner, and fees to the treasury
    let winner_transfer_messages =
        get_raffle_winner_messages(deps.as_ref(), env.clone(), raffle_info.clone(), raffle_id)?;
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
        return Err(ContractError::WrongStateForRandomness {
            status: raffle_state,
        });
    }
    // We assert the randomness is correct
    // get randomness from nois.network
    get_nois_randomness(deps.as_ref(), raffle_id)
}

#[allow(clippy::too_many_arguments)]
pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: Option<String>,
    owner: Option<String>,
    fee_addr: Option<String>,
    minimum_raffle_duration: Option<u64>,
    minimum_raffle_timeout: Option<u64>,
    max_tickets_per_raffle: Option<u32>,
    raffle_fee: Option<Decimal>,
    nois_proxy_addr: Option<String>,
    nois_proxy_coin: Option<Coin>,
    creation_coins: Option<Vec<Coin>>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // ensure msg sender is admin
    ensure_eq!(info.sender, config.owner, ContractError::Unauthorized);
    let name = match name {
        Some(n) => {
            if is_valid_name(&n) {
                n
            } else {
                config.name
            }
        }
        None => config.name,
    };
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
        Some(rf) => {
            ensure!(
                rf >= Decimal::zero() && rf <= Decimal::one(),
                ContractError::InvalidFeeRate {}
            );
            rf
        }
        None => config.raffle_fee,
    };

    let nois_proxy_addr = match nois_proxy_addr {
        Some(prx) => deps.api.addr_validate(&prx)?,
        None => config.nois_proxy_addr,
    };
    let nois_proxy_coin = match nois_proxy_coin {
        Some(npc) => {
            ensure!(
                npc.amount >= Uint128::zero(),
                ContractError::InvalidProxyCoin
            );
            npc
        }
        None => config.nois_proxy_coin,
    };

    // verifies all provided coins are greater than 0
    let creation_coins = match creation_coins {
        Some(mut crc) => {
            for coin in &mut crc {
                ensure!(
                    coin.amount >= Uint128::zero(),
                    ContractError::InvalidFeeRate {}
                )
            }
            crc
        }
        None => config.creation_coins,
    };
    let max_tickets_per_raffle = match max_tickets_per_raffle {
        Some(mpn) => mpn,
        None => config.max_tickets_per_raffle.unwrap(),
    };
    // we have a seperate function to lock a raffle, so we skip here

    let new_config = Config {
        name,
        owner,
        fee_addr,
        minimum_raffle_duration,
        minimum_raffle_timeout,
        raffle_fee,
        locks: config.locks,
        nois_proxy_addr,
        nois_proxy_coin,
        creation_coins,
        max_tickets_per_raffle: max_tickets_per_raffle.into(),
        last_raffle_id: config.last_raffle_id,
    };

    CONFIG.save(deps.storage, &new_config)?;

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

    config.locks.lock = lock;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "modify_parameter")
        .add_attribute("parameter", "contract_lock")
        .add_attribute("value", lock.to_string()))
}

// governance can lock contract
pub fn execute_sudo_toggle_lock(
    deps: DepsMut,
    _env: Env,
    lock: bool,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    config.locks.sudo_lock = lock;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "sudo_update_status")
        .add_attribute("parameter", "contract_lock")
        .add_attribute("value", lock.to_string()))
}
