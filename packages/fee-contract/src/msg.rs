use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdError, StdResult, Uint128};
use utils::state::{is_valid_name, AssetInfo};


#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub owner: Option<String>,
    pub p2p_contract: String,
    pub fee_distributor: String,
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
    PayFeeAndWithdraw {
        trade_id: u64,
    },
    UpdateFeeRates {
        asset_fee_rate: Option<Uint128>, // In thousandths (fee rate for liquid assets (terra native funds))
        fee_max: Option<Uint128>, // In uusd (max asset fee paid (outside of terra native funds))
        first_tier_limit: Option<Uint128>, // Max number of NFT to fall into the first tax tier
        first_tier_rate: Option<Uint128>, // Fee per asset in the first tier
        second_tier_limit: Option<Uint128>, // Max number of NFT to fall into the second tax tier
        second_tier_rate: Option<Uint128>, // Fee per asset in the second tier
        third_tier_rate: Option<Uint128>, // Fee per asset in the third tier
        acceptable_fee_deviation: Option<Uint128>, // To account for fluctuations in terra native prices, we allow the provided fee the deviate from the quoted fee (non simultaeous operations)
    },
}

#[cw_serde]
pub enum QueryMsg {
    Fee {
        trade_id: u64,
        counter_id: Option<u64>,
    },
    SimulateFee {
        trade_id: u64,
        counter_assets: Vec<AssetInfo>,
    },
    ContractInfo {},
    FeeRates {},
}

#[cw_serde]
pub struct FeeResponse {
    pub amount: Uint128,
    pub denom: String,
}

#[cw_serde]
pub struct FeeRawResponse {
    pub assets_fee: Uint128,
    pub funds_fee: Uint128,
}