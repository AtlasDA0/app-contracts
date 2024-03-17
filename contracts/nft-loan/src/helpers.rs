use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, BankMsg, Coin, Deps, StdResult, WasmMsg};
use utils::types::CosmosMsg;

use crate::{error::ContractError, msg::ExecuteMsg, state::CONFIG};

/// NftLoanContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[cw_serde]
pub struct NftLoanContract(pub Addr);

impl NftLoanContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }
}

pub fn assert_listing_fee(deps: Deps, funds: Vec<Coin>) -> Result<CosmosMsg, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let fee = funds
        .into_iter()
        .find(|c| config.listing_fee_coins.contains(c))
        .unwrap_or_default();

    if !config.listing_fee_coins.contains(&fee) {
        return Err(ContractError::DepositFeeError {});
    }
    // transfer fee to treasury_addr
    let transfer_fee_msg: CosmosMsg = BankMsg::Send {
        to_address: config.treasury_addr.to_string(),
        amount: vec![fee], // only the fee required
    }
    .into();

    Ok(transfer_fee_msg)
}
