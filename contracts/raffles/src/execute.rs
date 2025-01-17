use cosmwasm_std::{
    ensure, ensure_eq, Addr, BankMsg, Coin, Coins, Decimal, DepsMut, Empty, Env, MessageInfo,
    StdError, StdResult, Uint128,
};
use cw721::Cw721ExecuteMsg;
use cw721_base::Extension;

#[cfg(feature = "sg")]
use {crate::query::is_sg721_owner, sg721::ExecuteMsg as Sg721ExecuteMsg};

use utils::{
    state::{all_elements_unique, into_cosmos_msg, is_valid_comment, is_valid_name, AssetInfo},
    types::{CosmosMsg, Response},
};

use crate::{
    error::ContractError,
    msg::DrandConfig,
    query::is_nft_owner,
    state::{
        get_raffle_state, load_raffle, Config, FeeDiscountMsg, RaffleInfo, RaffleOptions,
        RaffleOptionsMsg, RaffleState, CONFIG, MINIMUM_RAFFLE_DURATION, RAFFLE_INFO,
        RAFFLE_TICKETS, USER_TICKETS,
    },
    utils::{
        buyer_can_buy_ticket, can_buy_ticket, get_raffle_owner_funds_finished_messages,
        get_raffle_owner_messages, get_raffle_refund_funds_finished_messages,
        get_raffle_winner_messages, get_raffle_winners, is_raffle_owner, ticket_cost,
    },
};

pub const NOIS_TIMEOUT: u64 = 6;

#[allow(clippy::too_many_arguments)]
pub fn execute_create_raffle(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Option<String>,
    all_assets: Vec<AssetInfo>,
    raffle_ticket_price: AssetInfo,
    raffle_options: RaffleOptionsMsg,
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

    let mut sent_coins: Coins = info.funds.try_into()?;

    // checks if the required fee was sent.
    let fee = sent_coins
        .iter()
        .find(|c| config.creation_coins.contains(c))
        .cloned()
        .unwrap_or_default();
    sent_coins.sub(fee.clone())?;

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

    // Make sure there is no duplicate
    if !all_elements_unique(&all_assets) {
        return Err(ContractError::DuplicateAssets {});
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

                into_cosmos_msg(message, token.address.clone(), None).map(Some)
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

                into_cosmos_msg(message, token.address.clone(), None).map(Some)
            }
            AssetInfo::Coin(native_coin) => {
                // We verify the tokens were just transfered
                sent_coins.sub(native_coin.clone())?;

                // There is no additional message to send there
                Ok(None)
            }
        })
        .collect::<Result<Vec<Option<CosmosMsg>>, StdError>>()?
        .into_iter()
        .flatten()
        .collect();

    // Then we create the internal raffle structure
    let owner = owner.map(|x| deps.api.addr_validate(&x)).transpose()?;
    // defines the fee token to send to nois-proxy, by the smart contract
    let raffle_id = _create_raffle(
        deps.branch(),
        env.clone(),
        owner.clone().unwrap_or_else(|| info.sender.clone()),
        all_assets,
        raffle_ticket_price,
        raffle_options.clone(),
    )?;

    let raffle_options = RAFFLE_INFO.load(deps.storage, raffle_id)?.raffle_options;
    let raffle_lifecycle = raffle_options
        .raffle_start_timestamp
        .plus_seconds(raffle_options.raffle_duration)
        .plus_seconds(NOIS_TIMEOUT);

    let mut msgs = vec![];

    // bypass sending fee if static raffle creation cost is 0
    if !fee.amount.is_zero() {
        // transfer only the calculated fee amount from the coins sent
        let transfer_fee_msg: CosmosMsg = BankMsg::Send {
            to_address: config.fee_addr.to_string(),
            amount: vec![fee],
        }
        .into();
        // add msg to response
        msgs.push(transfer_fee_msg);
    };

    Ok(Response::new()
        .add_messages(msgs)
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
            winners: vec![],
            is_cancelled: false,
            raffle_options: RaffleOptions::new(
                deps.api,
                env,
                all_assets.len(),
                raffle_options,
                config,
            )?,
            drand_randomness: None,
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
    let config = CONFIG.load(deps.storage)?;

    // The raffle can only be cancelled if it wasn't previously cancelled and it isn't finished
    let raffle_state = get_raffle_state(&env, &config, &raffle_info);

    if raffle_state != RaffleState::Created
        && raffle_state != RaffleState::Started
        && raffle_state != RaffleState::Closed
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
    let config = CONFIG.load(deps.storage)?;
    let mut raffle_info = is_raffle_owner(deps.storage, raffle_id, info.sender)?;
    let raffle_state = get_raffle_state(&env, &config, &raffle_info);
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
    on_behalf_of: Option<String>,
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
    let owner = on_behalf_of
        .map(|a| deps.as_ref().api.addr_validate(&a))
        .transpose()?
        .unwrap_or(info.sender.clone());
    _buy_tickets(deps, env.clone(), owner, raffle_id, ticket_count, assets)?;

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

    let config = CONFIG.load(deps.storage)?;
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

    // We start by checking that the buyer has the gating rights to buy this ticket
    buyer_can_buy_ticket(deps.as_ref(), &raffle_info, owner.to_string())?;

    // We then check the raffle is in the right state
    can_buy_ticket(env.clone(), &config, raffle_info.clone())?;

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

    // If all tickets have been bought, we stop the raffle.
    // The raffle duration is amended to reflect that
    // If not enough were bought before and we passed the threshold, we can send the randomness trigger as well
    if let Some(max_ticket_number) = raffle_info.raffle_options.max_ticket_number {
        if raffle_info.number_of_tickets >= max_ticket_number {
            raffle_info.raffle_options.raffle_duration = env.block.time.seconds()
                - raffle_info.raffle_options.raffle_start_timestamp.seconds();
        }
    }

    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    Ok(())
}

pub fn execute_claim(deps: DepsMut, env: Env, raffle_id: u64) -> Result<Response, ContractError> {
    let mut raffle_info = load_raffle(deps.storage, raffle_id)?;
    let config = CONFIG.load(deps.storage)?;
    let raffle_state = get_raffle_state(&env, &config, &raffle_info);

    if raffle_state != RaffleState::Finished {
        return Err(ContractError::WrongStateForClaim {
            status: raffle_state,
        });
    }
    // We determine the winner
    // Loads the raffle id and makes sure the raffle has ended and randomness from nois has been provided.

    // If there was no participant, the winner is the raffle owner
    // If the minimum number of tickets is not reached, the winner is the raffle owner as well
    let msgs = if raffle_info.number_of_tickets == 0u32 {
        raffle_info.winners = vec![raffle_info.owner.clone()];
        // No funds re-imbursement
        get_raffle_winner_messages(deps.as_ref(), env.clone(), raffle_info.clone())?
    } else if raffle_info.number_of_tickets
        < raffle_info.raffle_options.min_ticket_number.unwrap_or(0)
    {
        raffle_info.winners = vec![raffle_info.owner.clone()];
        // No funds re-imbursement
        let nft_msg = get_raffle_winner_messages(deps.as_ref(), env.clone(), raffle_info.clone())?;
        let refund_msgs = get_raffle_refund_funds_finished_messages(
            deps.storage,
            env.clone(),
            raffle_info.clone(),
            raffle_id,
        )?;
        [refund_msgs, nft_msg].concat()
    } else {
        // We calculate the winner of the raffle and save it to the contract. The raffle is now claimed !
        raffle_info.winners =
            get_raffle_winners(deps.as_ref(), &env, raffle_id, raffle_info.clone())?;
        let owner_funds_msg = get_raffle_owner_funds_finished_messages(
            deps.as_ref(),
            env.clone(),
            raffle_info.clone(),
        )?;
        let nft_msg = get_raffle_winner_messages(deps.as_ref(), env.clone(), raffle_info.clone())?;

        [owner_funds_msg, nft_msg].concat()
    };

    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    // We distribute the ticket prices to the owner and in part to the treasury
    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "claim")
        .add_attribute("raffle_id", raffle_id.to_string())
        .add_attribute(
            "winners",
            raffle_info
                .winners
                .into_iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        ))
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
    max_tickets_per_raffle: Option<u32>,
    raffle_fee: Option<Decimal>,
    drand_config: Option<DrandConfig>,
    creation_coins: Option<Vec<Coin>>,
    fee_discounts: Option<Vec<FeeDiscountMsg>>,
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

    let drand_config = match drand_config {
        Some(config) => {
            config.validate(deps.as_ref())?;
            config
        }
        None => config.drand_config,
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

    let fee_discounts = match fee_discounts {
        Some(discounts) => discounts
            .into_iter()
            .map(|d| d.check(deps.api))
            .collect::<Result<_, _>>()?,
        None => config.fee_discounts,
    };
    // we have a seperate function to lock a raffle, so we skip here

    let new_config = Config {
        name,
        owner,
        fee_addr,
        minimum_raffle_duration,
        raffle_fee,
        locks: config.locks,
        creation_coins,
        max_tickets_per_raffle: max_tickets_per_raffle.into(),
        last_raffle_id: config.last_raffle_id,
        fee_discounts,
        drand_config,
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
