use cosmwasm_std::Empty;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};

use drand_verify::{derive_randomness, g1_from_variable, verify};
use randomness::DrandRandomness;
pub use randomness::VerifierExecuteMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    // store token info
    Ok(Response::default().add_attribute("fee_contract", "randomness_verifier"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: VerifierExecuteMsg,
) -> StdResult<Response> {
    match msg {
        VerifierExecuteMsg::Verify {
            randomness,
            pubkey,
            raffle_id,
            owner,
        } => execute_verify(randomness, pubkey, raffle_id, owner),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    Err(StdError::generic_err("No queries"))
}

pub fn execute_verify(
    randomness: DrandRandomness,
    pubkey: Binary,
    raffle_id: u64,
    owner: String,
) -> StdResult<Response> {
    let pk = g1_from_variable(&pubkey).map_err(|_| StdError::generic_err("Invalid Public Key"))?;
    let valid = verify(
        &pk,
        randomness.round,
        randomness.previous_signature.as_slice(),
        randomness.signature.as_slice(),
    )
    .unwrap_or(false);

    if !valid {
        return Err(StdError::generic_err("Invalid Signature"));
    }

    let randomness_result = derive_randomness(&randomness.signature);

    Ok(Response::new()
        .add_attribute("round", randomness.round.to_string())
        .add_attribute("randomness", Binary::from(randomness_result).to_string())
        .add_attribute("raffle_id", raffle_id.to_string())
        .add_attribute("owner", owner))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    extern crate rustc_serialize as serialize;

    use randomness::VerifierExecuteMsg;
    use serialize::base64::{self, ToBase64};
    use serialize::hex::FromHex;
    const HEX_PUBKEY: &str = "868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31";

    fn init_helper(deps: DepsMut) {
        let instantiate_msg = Empty {};
        let info = mock_info("creator", &[]);
        let env = mock_env();

        instantiate(deps, env, info, instantiate_msg).unwrap();
    }

    #[test]
    fn verify() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("caller", &[]);
        init_helper(deps.as_mut());

        let randomness = DrandRandomness{
            round:2098475,
            signature:Binary::from_base64(&"84bcb438f4505ef5282804c3d98a76b536970f9755004ea103fc15183c1b314a1e95582cb4dbaf3330272c72fe5675550ac6718ef53dfe18fca65c37f223a7dfbc1920a9c32dbf01a227290293a67fa4682dd266f717f5078829d83912926649".from_hex().unwrap().to_base64(base64::STANDARD)).unwrap(),
            previous_signature:Binary::from_base64(&"b49ee4089fc510300b38d75ebba84576097bed61c171574acf2557f636c7144b471b57e18e5b0c3f8774e194344931c011873149d0db51fc70d22448bfc264d230be7ed6fcd3eb3b61fdc877d657dfa0d8ecaea6c1fa35f90bc84e88c1af17d4".from_hex().unwrap().to_base64(base64::STANDARD)).unwrap(),
        };

        let pubkey =
            Binary::from_base64(&HEX_PUBKEY.from_hex().unwrap().to_base64(base64::STANDARD))
                .unwrap();

        let raffle_id = 0u64;
        let owner = "anyone".to_string();
        let response = execute(
            deps.as_mut(),
            env,
            info,
            VerifierExecuteMsg::Verify {
                randomness,
                pubkey,
                raffle_id,
                owner,
            },
        )
        .unwrap();
        assert_eq!(
            response,
            Response::new()
                .add_attribute("round", 2098475u128.to_string())
                .add_attribute("randomness", "iVgPamOa3WyQ3PPSIuNUFfidnuLNbvb8TyMTTN/6XR4=")
                .add_attribute("raffle_id", 0u128.to_string())
                .add_attribute("owner", "anyone")
        );
    }
}
