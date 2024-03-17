use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coins, to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, HexBinary, MessageInfo,
    StdError, StdResult, Timestamp,
};
use cw_storage_plus::{Item, Map, Prefixer};
use nois::{NoisCallback, ProxyExecuteMsg};
use utils::types::Response;

#[cw_serde]
pub struct Config {
    nois: String,
}

#[cw_serde]
pub struct RandomnessForLater {
    after: Timestamp,
}

const CONFIG: Item<Config> = Item::new("config");
const RANDOMNESS: Map<(Addr, String), RandomnessForLater> = Map::new("randomnesss");

#[cw_serde]
pub struct InstantiateMsg {}
#[cw_serde]
pub struct QueryMsg {}

pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, StdError> {
    Ok(Response::new())
}

pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> Result<Binary, StdError> {
    panic!()
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ProxyExecuteMsg,
) -> Result<Response, StdError> {
    match msg {
        ProxyExecuteMsg::GetNextRandomness { job_id } => {
            resubmit_randomness_right_now(deps, env, info, job_id)
        }
        ProxyExecuteMsg::GetRandomnessAfter { after, job_id } => {
            register_randmoness_for_later(deps, env, info, after, job_id)
        }
    }
}

fn resubmit_randomness_right_now(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    job_id: String,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if info.funds != coins(1_000_000, config.nois) {
        return Err(StdError::generic_err("Nois not enough funds sent to proxy"));
    }

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
            contract_addr: info.sender.to_string(),
            msg: to_json_binary(&raffles::msg::ExecuteMsg::NoisReceive {
                callback: NoisCallback {
                    job_id,
                    published: env.block.time,
                    randomness: HexBinary::from_hex(
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa115",
                    )?,
                },
            })?,
            funds: vec![],
        })),
    )
}

pub fn register_randmoness_for_later(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    after: Timestamp,
    job_id: String,
) -> Result<Response, StdError> {
    if let Some(job_id) = job_id.strip_prefix("test-trigger-") {
        // This is a bypass for the test contract to trigger the randomness
        let job = RANDOMNESS.load(deps.storage, (info.sender.clone(), job_id.to_string()))?;

        // Make sure we are after the job_id
        if env.block.time < job.after {
            return Err(StdError::generic_err(
                "Too soon to test-trigger randomness providing",
            ));
        }

        // Remove job
        RANDOMNESS.remove(deps.storage, (info.sender.clone(), job_id.to_string()));
        Ok(
            Response::new().add_message(CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: info.sender.to_string(),
                msg: to_json_binary(&raffles::msg::ExecuteMsg::NoisReceive {
                    callback: NoisCallback {
                        job_id: job_id.to_string(),
                        published: env.block.time,
                        randomness: HexBinary::from_hex(
                            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa115",
                        )?,
                    },
                })?,
                funds: vec![],
            })),
        )
    } else {
        // Here we just register the randomnesss for later
        RANDOMNESS.save(
            deps.storage,
            (info.sender, job_id),
            &RandomnessForLater { after },
        )?;
        Ok(Response::new())
    }
}
