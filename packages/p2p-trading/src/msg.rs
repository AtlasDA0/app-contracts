use crate::state::{Comment, CounterTradeInfo, TradeInfo, TradeState};
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Coin, CosmosMsg, Decimal, StdError, StdResult, Timestamp, WasmMsg,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::iter::FromIterator;
use utils::state::AssetInfo;

fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 50 {
        return false;
    }
    true
}

pub fn into_json_binary<M: Serialize>(msg: M) -> StdResult<Binary> {
    to_json_binary(&msg)
}

pub fn into_cosmos_msg<M: Serialize, T: Into<String>>(
    message: M,
    contract_addr: T,
) -> StdResult<CosmosMsg> {
    let msg = into_json_binary(message)?;
    let execute = WasmMsg::Execute {
        contract_addr: contract_addr.into(),
        msg,
        funds: vec![],
    };
    Ok(execute.into())
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub name: String,
    pub owner: Option<String>,
    pub accept_trade_fee: Vec<Coin>,
    pub fund_fee: Decimal,
    pub treasury: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct MigrateMsg {}

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AddAssetAction {
    ToLastTrade {},
    ToLastCounterTrade { trade_id: u64 },
    ToTrade { trade_id: u64 },
    ToCounterTrade { trade_id: u64, counter_id: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    CreateTrade {
        whitelisted_users: Option<Vec<String>>,
        comment: Option<String>,
    },
    #[cw_orch(payable)]
    AddAsset {
        action: AddAssetAction,
        asset: AssetInfo,
    },
    RemoveAssets {
        trade_id: u64,
        counter_id: Option<u64>,
        assets: Vec<(u16, AssetInfo)>,
    },
    AddWhitelistedUsers {
        trade_id: u64,
        whitelisted_users: Vec<String>,
    },
    RemoveWhitelistedUsers {
        trade_id: u64,
        whitelisted_users: Vec<String>,
    },
    SetComment {
        trade_id: u64,
        counter_id: Option<u64>,
        comment: String,
    },
    AddNFTsWanted {
        trade_id: Option<u64>,
        nfts_wanted: Vec<String>,
    },
    RemoveNFTsWanted {
        trade_id: u64,
        nfts_wanted: Vec<String>,
    },
    SetNFTsWanted {
        trade_id: Option<u64>,
        nfts_wanted: Vec<String>,
    },
    FlushNFTsWanted {
        trade_id: u64,
    },

    AddTokensWanted {
        trade_id: Option<u64>,
        tokens_wanted: Vec<Coin>,
    },
    RemoveTokensWanted {
        trade_id: u64,
        tokens_wanted: Vec<Coin>,
    },
    SetTokensWanted {
        trade_id: Option<u64>,
        tokens_wanted: Vec<Coin>,
    },
    FlushTokensWanted {
        trade_id: u64,
    },

    // Sets an NFT as the preview of the trade
    // This is only informational and has no effect on the trade
    SetTradePreview {
        action: AddAssetAction,
        asset: AssetInfo,
    },

    /// Is used by the Trader to confirm they completed their end of the trade.
    ConfirmTrade {
        trade_id: Option<u64>,
    },
    /// Can be used to initiate Counter Trade, but also to add new tokens to it
    SuggestCounterTrade {
        trade_id: u64,
        comment: Option<String>,
    },
    /// Is used by the Client to confirm they completed their end of the trade.
    ConfirmCounterTrade {
        trade_id: u64,
        counter_id: Option<u64>,
    },
    /// Accept the Trade plain and simple, swap it up !
    AcceptTrade {
        trade_id: u64,
        counter_id: u64,
        comment: Option<String>,
    },
    /// Cancel the Trade :/ No luck there mate ?
    CancelTrade {
        trade_id: u64,
    },
    /// Cancel the Counter Trade :/ No luck there mate ?
    CancelCounterTrade {
        trade_id: u64,
        counter_id: u64,
    },
    /// Refuse the Trade plain and simple, no madam, I'm not interested in your tokens !
    RefuseCounterTrade {
        trade_id: u64,
        counter_id: u64,
    },
    /// Some parts of the traded tokens were interesting, but you can't accept the trade as is
    ReviewCounterTrade {
        trade_id: u64,
        counter_id: u64,
        comment: Option<String>,
    },
    #[cw_orch(payable)]
    /// The trader or counter trader contract can Withdraw funds via this function only when the trade is accepted.
    WithdrawSuccessfulTrade {
        trade_id: u64,
    },
    /// You can Withdraw funds only at specific steps of the trade, but you're allowed to try anytime !
    WithdrawAllFromTrade {
        trade_id: u64,
    },
    /// You can Withdraw funds when your counter trade is aborted (refused or cancelled)
    /// Or when you are creating the trade and you just want to cancel it all
    WithdrawAllFromCounter {
        trade_id: u64,
        counter_id: u64,
    },

    /// Direct Buy
    DirectBuy {
        trade_id: u64,
    },

    // Admin operations //
    SetNewOwner {
        owner: String,
    },
    SetNewTreasury {
        treasury: String,
    },
    SetNewAcceptFee {
        accept_fee: Vec<Coin>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub struct QueryFilters {
    pub states: Option<Vec<String>>,
    pub owner: Option<String>,
    pub counterer: Option<String>,
    pub has_whitelist: Option<bool>,
    pub whitelisted_user: Option<String>,
    pub contains_token: Option<String>,
    pub wanted_nft: Option<String>,
    pub assets_withdrawn: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ContractInfo {},
    TradeInfo {
        trade_id: u64,
    },
    CounterTradeInfo {
        trade_id: u64,
        counter_id: u64,
    },
    GetAllTrades {
        start_after: Option<u64>,
        limit: Option<u32>,
        filters: Option<QueryFilters>,
    },
    GetCounterTrades {
        trade_id: u64,
        start_after: Option<u64>,
        limit: Option<u32>,
        filters: Option<QueryFilters>,
    },
    GetAllCounterTrades {
        start_after: Option<CounterTradeInfo>,
        limit: Option<u32>,
        filters: Option<QueryFilters>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub struct AdditionalTradeInfoResponse {
    pub time: Timestamp,
    pub owner_comment: Option<Comment>,
    pub trader_comment: Option<Comment>,
    pub nfts_wanted: Vec<Addr>,
    pub tokens_wanted: Vec<Coin>, // The tokens wanted can only be a coin
    pub trade_preview: Option<AssetInfo>, // The preview can only be a CW1155 or a CW721 token.
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TradeInfoResponse {
    pub owner: Addr,
    pub associated_assets: Vec<AssetInfo>,
    pub state: TradeState,
    pub last_counter_id: Option<u64>,
    pub whitelisted_users: Vec<Addr>,
    pub additional_info: AdditionalTradeInfoResponse,
    pub accepted_info: Option<CounterTradeInfo>,
    pub assets_withdrawn: bool,
}

impl TryFrom<TradeInfo> for TradeInfoResponse {
    type Error = StdError;

    fn try_from(trade_info: TradeInfo) -> StdResult<Self> {
        Ok(TradeInfoResponse {
            owner: trade_info.owner,
            associated_assets: trade_info.associated_assets,
            state: trade_info.state,
            last_counter_id: trade_info.last_counter_id,
            whitelisted_users: Vec::from_iter(trade_info.whitelisted_users),
            additional_info: AdditionalTradeInfoResponse {
                time: trade_info.additional_info.time,
                owner_comment: trade_info.additional_info.owner_comment,
                trader_comment: trade_info.additional_info.trader_comment,
                nfts_wanted: Vec::from_iter(trade_info.additional_info.nfts_wanted),
                tokens_wanted: trade_info.additional_info.tokens_wanted,
                trade_preview: trade_info.additional_info.trade_preview,
            },
            accepted_info: trade_info.accepted_info,
            assets_withdrawn: trade_info.assets_withdrawn,
        })
    }
}

impl Default for TradeInfoResponse {
    fn default() -> Self {
        Self {
            owner: Addr::unchecked(""),
            associated_assets: vec![],
            state: TradeState::Created,
            last_counter_id: None,
            whitelisted_users: vec![],
            additional_info: AdditionalTradeInfoResponse::default(),
            accepted_info: None,
            assets_withdrawn: false,
        }
    }
}
