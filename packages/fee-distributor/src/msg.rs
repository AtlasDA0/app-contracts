use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdError, StdResult, Uint128};
use fee_contract_export::state::FeeType;
use utils::state::is_valid_name;


#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub owner: Option<String>,
    pub treasury: String,
}

impl InstantiateMsg {
    pub fn validate(&self) -> StdResult<()> {
        // Check name, symbol, decimals
        if !is_valid_name(&self.name) {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        Ok(())
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    ModifyContractInfo {
        owner: Option<String>,
        treasury: Option<String>,
        projects_allocation_for_assets_fee: Option<Uint128>,
        projects_allocation_for_funds_fee: Option<Uint128>,
    },
    DepositFees {
        addresses: Vec<String>,
        fee_type: FeeType,
    },
    WithdrawFees {
        addresses: Vec<String>,
    },
    AddAssociatedAddress {
        address: String,
        fee_address: String,
    },
}

#[cw_serde]
pub enum QueryMsg {
    ContractInfo {},
    Amount {
        address: String,
    },
    Addresses {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}
