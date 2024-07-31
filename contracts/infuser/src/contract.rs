#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env,
    HexBinary, MessageInfo, QueryRequest, Reply, Response, StdResult, Storage, SubMsg, WasmMsg,
    WasmQuery,
};
use cw2::set_contract_version;
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, OwnerOfResponse};
use cw721_base::{ExecuteMsg as Cw721ExecuteMessage, InstantiateMsg as Cw721InstantiateMsg};
use cw_controllers::AdminError;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InfusionsResponse, InstantiateMsg, QueryMsg};
use crate::state::{
    Bundle, Config, InfusedCollection, Infusion, InfusionInfo, CONFIG, INFUSION, INFUSION_ID,
    INFUSION_INFO, NFT,
};
use cosmwasm_schema::serde::Serialize;

const INFUSION_COLLECTION_INIT_MSG_ID: u64 = 21;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-infuser";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(
        deps.storage,
        &Config {
            min_per_bundle: 1u64,
            max_per_bundle: msg.max_token_in_bundle.unwrap_or(10u64),
            code_id: msg.cw721_code_id,
            latest_infusion_id: None,
            admin: info.sender,
            max_infusions: msg.max_infusions.unwrap_or(2u64),
            max_bundles: msg.max_bundles.unwrap_or(5),
        },
    )?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateInfusion { infusions } => {
            execute_create_infusion(deps, info, env, infusions)
        }
        ExecuteMsg::Infuse {
            infusion_id,
            bundle,
        } => execute_infuse_bundle(deps, info, infusion_id, bundle),
        ExecuteMsg::UpdateConfig {
            admin,
            max_infusions,
            min_infusions_per_bundle,
            max_infusions_per_bundle,
        } => update_config(
            deps,
            info,
            admin,
            max_infusions,
            min_infusions_per_bundle,
            max_infusions_per_bundle,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Infusion { addr, id } => to_json_binary(&query_infusion(deps, addr, id)?),
        QueryMsg::InfusionById { id } => to_json_binary(&query_infusion_by_id(deps, id)?),
        QueryMsg::Infusions { addr } => to_json_binary(&query_infusions(deps, addr)?),
        QueryMsg::IsInBundle {
            id,
            collection_addr,
        } => to_json_binary(&query_is_in_infusion(deps, id, collection_addr)?),
        QueryMsg::InfusedCollection { id } => to_json_binary(&query_infused_collection(deps, id)?),
    }
}

pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        INFUSION_COLLECTION_INIT_MSG_ID => match msg.result {
            cosmwasm_std::SubMsgResult::Ok(_) => Ok(Response::new()),
            cosmwasm_std::SubMsgResult::Err(err) => {
                Ok(Response::new().add_attribute("infusion_creation_error", err.to_string()))
            }
        },
        _ => panic!("unexpected nois mock proxy reply"),
    }
}

/// Update the configuration of the app
fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
    max_infusions: Option<u64>,
    max_per_bundle: Option<u64>,
    min_per_bundle: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    // Only the admin should be able to call this
    if config.admin != info.sender {
        return Err(ContractError::Admin(AdminError::NotAdmin {}));
    }
    if let Some(new) = admin {
        config.admin = deps.api.addr_validate(&new)?;
    }
    if let Some(new) = max_infusions {
        config.max_infusions = new;
    }
    if let Some(new) = min_per_bundle {
        config.min_per_bundle = new;
    }
    if let Some(new) = max_per_bundle {
        config.max_per_bundle = new;
    }

    //todo: update configs

    Ok(Response::new())
}

pub fn execute_create_infusion(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    infusions: Vec<Infusion>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut msgs: Vec<SubMsg> = Vec::new();

    if infusions.len() > config.max_infusions.try_into().unwrap() {
        return Err(ContractError::TooManyInfusions {});
    }
    // creates the infused collection salt via wasm blob checksum to accurately predict address before its instantiated
    let collection_checksum = deps.querier.query_wasm_code_info(config.code_id)?.checksum;
    let salt1 = generate_instantiate_salt2(&collection_checksum);

    // loop through each infusion being created to assert its configuration
    for infusion in infusions {
        // checks # of bundles
        if config.max_bundles < infusion.collections.len().try_into().unwrap() {
            return Err(ContractError::TooManyBudlesDefined {
                want: config.max_bundles,
                got: infusion.collections.len().try_into().unwrap(),
            });
        }
        // check in each nft collections wanted amnt is within contract params
        for c in infusion.collections.clone() {
            // checks # of nft required per collection
            if config.min_per_bundle > c.min_wanted || config.max_per_bundle < c.min_wanted {
                return Err(ContractError::BadBundle {
                    have: c.min_wanted,
                    min: config.min_per_bundle,
                    max: config.max_per_bundle,
                });
            }
        }

        // predict the contract address
        let infusion_addr = match instantiate2_address(
            collection_checksum.as_slice(),
            &deps.api.addr_canonicalize(env.contract.address.as_str())?,
            salt1.as_slice(),
        ) {
            Ok(addr) => addr,
            Err(err) => return Err(ContractError::from(err)),
        };

        let infusion_collection_addr_human = deps.api.addr_humanize(&infusion_addr)?;
        // get the global infusion id
        let infusion_id: u64 = CONFIG
            .update(deps.storage, |mut c| -> StdResult<_> {
                c.latest_infusion_id = c.latest_infusion_id.map_or(Some(0), |id| Some(id + 1));
                Ok(c)
            })?
            .latest_infusion_id
            .unwrap();

        // sets infuser contract as admin if no admin specified (not sure if we want this)
        let admin = Some(
            infusion
                .infused_collection
                .admin
                .unwrap_or(env.contract.address.to_string()),
        );

        let infusion_config = Infusion {
            collections: infusion.collections,
            infused_collection: InfusedCollection {
                addr: infusion_collection_addr_human,
                admin: admin.clone(),
                name: infusion.infused_collection.name.clone(),
                symbol: infusion.infused_collection.symbol.to_string(),
            },
            infusion_params: infusion.infusion_params,
            infusion_id,
        };

        let init_msg = Cw721InstantiateMsg {
            name: infusion.infused_collection.name.clone(),
            symbol: infusion.infused_collection.symbol,
            minter: env.contract.address.to_string(),
        };

        let init_infusion = WasmMsg::Instantiate2 {
            admin: admin.clone(),
            code_id: config.code_id,
            msg: to_json_binary(&init_msg)?,
            funds: vec![],
            label: "infused".to_string() + infusion.infused_collection.name.as_ref(),
            salt: salt1.clone(),
        };

        let infusion_collection_submsg =
            SubMsg::<Empty>::reply_on_success(init_infusion, INFUSION_COLLECTION_INIT_MSG_ID);

        // gets the next id for an address
        let id = get_next_id(deps.storage, info.sender.clone())?;

        // saves the infusion bundle to state with (creator, id)
        let key = (info.sender.clone(), id);
        INFUSION.save(deps.storage, key.clone(), &infusion_config)?;
        INFUSION_ID.save(deps.storage, infusion_id, &key)?;

        msgs.push(infusion_collection_submsg)
    }

    Ok(Response::new().add_submessages(msgs))
}

fn execute_infuse_bundle(
    deps: DepsMut,
    info: MessageInfo,
    infusion_id: u64,
    bundle: Vec<Bundle>,
) -> Result<Response, ContractError> {
    let res = Response::new();
    let mut msgs: Vec<CosmosMsg> = Vec::new();

    for bundle in bundle {
        let sender = info.sender.clone();
        // confirm correct # of nfts
        // confirms ownership for each nft in bundle
        is_nft_owner(deps.as_ref(), sender.clone(), bundle.nfts.clone())?;

        // burns nfts in each bundle, mint infused token also
        let messages = burn_bundle(deps.storage, sender, bundle.nfts, infusion_id)?;
        // add msgs to response
        msgs.extend(messages)
    }

    Ok(res.add_messages(msgs))
}

// burns all nft bundles
fn burn_bundle(
    storage: &mut dyn Storage,
    sender: Addr,
    nfts: Vec<NFT>,
    id: u64,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let _config = CONFIG.load(storage)?;
    println!("burn bundle");
    let key = INFUSION_ID.load(storage, id)?;
    let infusion = INFUSION.load(storage, key)?;

    // confirm bundle is in current infusion
    check_bundles(storage, id, nfts.clone())?;

    let mut messages: Vec<CosmosMsg> = Vec::new();
    for nft in nfts {
        let token_id = nft.token_id;
        let address = nft.addr;

        let message = Cw721ExecuteMsg::Burn {
            token_id: token_id.to_string(),
        };
        let msg = into_cosmos_msg(message, address, None)?;
        messages.push(msg);
    }

    // increment tokens
    let token_id = get_next_id(storage, infusion.infused_collection.addr.clone())?;

    // mint_msg
    let mint_msg = Cw721ExecuteMessage::<Empty, Empty>::Mint {
        token_id: token_id.to_string(),
        owner: sender.to_string(),
        token_uri: None,
        extension: Empty {},
    };

    let msg = into_cosmos_msg(mint_msg, infusion.infused_collection.addr, None)?;

    messages.push(msg);

    Ok(messages)
}

fn check_bundles(
    storage: &mut dyn Storage,
    id: u64,
    bundle: Vec<NFT>,
) -> Result<(), ContractError> {
    // get the InfusionConfig
    let key = INFUSION_ID.load(storage, id)?;
    let infusion = INFUSION.load(storage, key)?;

    // verify that the bundle is include in i
    for nft in &infusion.collections {
        let mut count = 0u64;

        bundle.iter().for_each(|t| {
            if t.addr == nft.addr {
                count = count + 1;
            }
        });

        if count.eq(&0) {
            return Err(ContractError::BundleNotAccepted);
        }

        if count < nft.min_wanted {
            return Err(ContractError::NotEnoughNFTsInBundle {
                a: nft.addr.to_string(),
            });
        }

        if let Some(max) = nft.max {
            if count > max {
                return Err(ContractError::TooManyNFTsInBundle {
                    a: nft.addr.to_string(),
                });
            }
        }
    }

    Ok(())
}

pub fn into_cosmos_msg<M: Serialize, T: Into<String>>(
    message: M,
    contract_addr: T,
    funds: Option<Vec<Coin>>,
) -> StdResult<CosmosMsg> {
    let msg = to_json_binary(&message)?;
    let execute = WasmMsg::Execute {
        contract_addr: contract_addr.into(),
        msg,
        funds: funds.unwrap_or_default(),
    };
    Ok(execute.into())
}

fn get_next_id(storage: &mut dyn Storage, addr: Addr) -> Result<u64, ContractError> {
    let token_id = INFUSION_INFO
        .update::<_, ContractError>(storage, &addr, |x| match x {
            Some(mut info) => {
                info.next_id += 1;
                Ok(info)
            }
            None => Ok(InfusionInfo::default()),
        })?
        .next_id;
    Ok(token_id)
}

pub fn get_current_id(storage: &mut dyn Storage, addr: Addr) -> Result<u64, ContractError> {
    let token_id = INFUSION_INFO.load(storage, &addr)?.next_id;
    Ok(token_id)
}

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config: Config = CONFIG.load(deps.storage)?;
    Ok(config)
}

pub fn query_infusion(deps: Deps, addr: Addr, id: u64) -> StdResult<Infusion> {
    let infusion = INFUSION.load(deps.storage, (addr, id))?;
    Ok(infusion)
}
pub fn query_infusion_by_id(deps: Deps, id: u64) -> StdResult<Infusion> {
    let infuser = INFUSION_ID.load(deps.storage, id)?;
    let infusion = INFUSION.load(deps.storage, infuser)?;
    Ok(infusion)
}
pub fn query_infused_collection(deps: Deps, id: u64) -> StdResult<InfusedCollection> {
    let infuser = INFUSION_ID.load(deps.storage, id)?;
    let infusion = INFUSION.load(deps.storage, infuser)?;
    Ok(infusion.infused_collection)
}

pub fn query_is_in_infusion(deps: Deps, id: u64, addr: Addr) -> StdResult<bool> {
    let infuser = INFUSION_ID.load(deps.storage, id)?;
    let infusion = INFUSION.load(deps.storage, infuser)?;
    for nfts in infusion.collections {
        if nfts.addr == addr {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn query_infusions(deps: Deps, addr: Addr) -> StdResult<InfusionsResponse> {
    let mut infusions = vec![];
    let current_id = INFUSION_INFO.load(deps.storage, &addr.clone())?.next_id;

    for i in 0..=current_id {
        let id = i;
        // return the response for each
        let state = INFUSION.load(deps.storage, (addr.clone(), id))?;
        infusions.push(state);
    }

    Ok(InfusionsResponse { infusions })
}

/// Generates the value used with instantiate2, via a hash of the infusers checksum.
pub const SALT_POSTFIX: &[u8] = b"infusion";
pub fn generate_instantiate_salt2(checksum: &HexBinary) -> Binary {
    let checksum_hash = <sha2::Sha256 as sha2::Digest>::digest(checksum.to_string());
    let mut hash = checksum_hash.to_vec();
    hash.extend(SALT_POSTFIX);
    Binary(hash.to_vec())
}

/// verifies all nfts defined in bundle are of ownership of the current sender
pub fn is_nft_owner(deps: Deps, sender: Addr, nfts: Vec<NFT>) -> Result<(), ContractError> {
    for nft in nfts {
        let nft_address = nft.addr;
        let token_id = nft.token_id;

        let owner_response: OwnerOfResponse =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: nft_address.to_string(),
                msg: to_json_binary(&Cw721QueryMsg::OwnerOf {
                    token_id: token_id.to_string(),
                    include_expired: None,
                })?,
            }))?;

        if owner_response.owner != sender {
            return Err(ContractError::SenderNotOwner {});
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {}
