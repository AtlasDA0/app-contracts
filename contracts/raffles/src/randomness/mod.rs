use crate::contract::VERIFY_RANDOMNESS_REPLY_ID;
use crate::error::ContractError;
use crate::state::{get_raffle_state, load_raffle, RaffleState, CONFIG, RAFFLE_INFO};
use cosmwasm_std::{
    wasm_execute, Addr, Binary, Deps, DepsMut, Env, Event, MessageInfo, SubMsg, SubMsgResult,
};
use randomness::{DrandRandomness, Randomness, VerifierExecuteMsg};
use sg_std::Response;

/// Update the randomness assigned to a raffle
/// The function receives and checks the randomness against the drand public_key registered with the account.
/// This allows trustless and un-predictable randomness to the raffle contract.
/// The randomness providers will get a small cut of the raffle tickets (to reimburse the tx fees and incentivize adding randomness)
pub fn execute_update_randomness(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    raffle_id: u64,
    randomness: DrandRandomness,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // We check the raffle can receive randomness (good state)
    let raffle_info = load_raffle(deps.storage, raffle_id)?;
    let raffle_state = get_raffle_state(&env, &config, &raffle_info);
    if raffle_state != RaffleState::Closed {
        return Err(ContractError::WrongStateForRandomness {
            status: raffle_state,
        });
    }
    // We assert the randomness is correct
    assert_randomness_origin_and_order(deps.as_ref(), info.sender, raffle_id, randomness)
}

/// This function is largely inspired (and even directly copied) from https://github.com/LoTerra/terrand-contract-step1/
/// This function actually simply calls an external contract that checks the randomness origin
/// This architecture was chosen because the imported libraries needed to verify signatures are very heavy
/// and won't upload when combined with the current contract.
/// Separating into 2 contracts seems to help with that
/// For more info about randomness, visit : https://drand.love/
pub fn assert_randomness_origin_and_order(
    deps: Deps,
    owner: Addr,
    raffle_id: u64,
    randomness: DrandRandomness,
) -> Result<Response, ContractError> {
    let raffle_info = load_raffle(deps.storage, raffle_id)?;
    let contract_info = CONFIG.load(deps.storage)?;

    if let Some(local_randomness) = raffle_info.drand_randomness {
        if randomness.round <= local_randomness.randomness_round {
            return Err(ContractError::RandomnessNotAccepted {
                current_round: local_randomness.randomness_round,
            });
        }
    }

    let msg = VerifierExecuteMsg::Verify {
        randomness,
        pubkey: contract_info.drand_config.random_pubkey,
        raffle_id,
        owner: owner.to_string(),
    };
    let verify_message = wasm_execute(
        contract_info.drand_config.verify_signature_contract,
        &msg,
        vec![],
    )?;

    let msg = SubMsg::reply_on_success(verify_message, VERIFY_RANDOMNESS_REPLY_ID);
    Ok(Response::new().add_submessage(msg))
}

/// This function is called after the randomness verifier has verified the current randomness
/// We used this architecture to make sure the verification passes (because a query may return early)
/// We verify the randomness provided matches the current state of the contract (good round, good raffle_id...)
/// We also save the new randomness in the contract
pub fn verify_randomness(
    deps: DepsMut,
    _env: Env,
    msg: SubMsgResult,
) -> Result<Response, ContractError> {
    let subcall = msg.into_result().unwrap();

    let event: Event = subcall
        .events
        .into_iter()
        .find(|e| e.ty == "wasm")
        .ok_or_else(|| ContractError::NotFoundError("wasm results".to_string()))?;

    let round = event
        .attributes
        .clone()
        .into_iter()
        .find(|attr| attr.key == "round")
        .map_or(
            Err(ContractError::NotFoundError("randomness round".to_string())),
            |round| {
                round
                    .value
                    .parse::<u64>()
                    .map_err(|_| ContractError::ParseError("randomness round".to_string()))
            },
        )?;

    let randomness: String = event
        .attributes
        .clone()
        .into_iter()
        .find(|attr| attr.key == "randomness")
        .map(|rand| rand.value)
        .ok_or_else(|| ContractError::NotFoundError("randomness value".to_string()))?;

    let raffle_id: u64 = event
        .attributes
        .clone()
        .into_iter()
        .find(|attr| attr.key == "raffle_id")
        .map_or(
            Err(ContractError::NotFoundError("raffle_id".to_string())),
            |raffle_id| {
                raffle_id
                    .value
                    .parse::<u64>()
                    .map_err(|_| ContractError::ParseError("raffle_id".to_string()))
            },
        )?;

    let owner = deps.api.addr_validate(
        &event
            .attributes
            .into_iter()
            .find(|attr| attr.key == "owner")
            .map(|owner| owner.value)
            .ok_or_else(|| ContractError::NotFoundError("randomness provider".to_string()))?,
    )?;

    let mut raffle_info = load_raffle(deps.storage, raffle_id)?;
    raffle_info.drand_randomness = Some(Randomness {
        randomness: Binary::from_base64(&randomness)?
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::ParseError("randomness".to_string()))?,
        randomness_round: round,
        randomness_owner: owner.clone(),
    });

    RAFFLE_INFO.save(deps.storage, raffle_id, &raffle_info)?;

    Ok(Response::new()
        .add_attribute("action", "update_randomness")
        .add_attribute("raffle_id", raffle_id.to_string())
        .add_attribute("sender", owner))
}
