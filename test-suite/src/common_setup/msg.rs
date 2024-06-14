use crate::common_setup::app::StargazeApp;
use anyhow::Error;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};
use sg2::msg::CollectionParams;
use vending_factory::msg::VendingMinterInitMsgExtension;

pub struct MinterCollectionResponse {
    pub minter: Option<Addr>,
    pub collection: Option<Addr>,
    pub factory: Option<Addr>,
    pub error: Option<Error>,
}

pub struct CreateRaffleResponse {
    pub raffle: Option<Addr>,
    pub owner: Option<Addr>,
    pub error: Option<Error>,
}

pub struct MinterSetupParams<'a> {
    pub router: &'a mut StargazeApp,
    pub minter_admin: Addr,
    pub num_tokens: u32,
    pub collection_params: CollectionParams,
    pub splits_addr: Option<String>,
    pub start_time: Option<Timestamp>,
    pub minter_code_id: u64,
    pub factory_code_id: u64,
    pub sg721_code_id: u64,
    pub init_msg: Option<VendingMinterInitMsgExtension>,
}

pub struct LoanSetupParams<'a> {
    pub router: &'a mut StargazeApp,
    pub loan_code_id: u64,
}
pub struct RaffleSetupParams<'a> {
    pub router: &'a mut StargazeApp,
}
pub struct MinterInstantiateParams {
    pub num_tokens: u32,
    pub start_time: Option<Timestamp>,
    pub splits_addr: Option<String>,
    pub init_msg: Option<VendingMinterInitMsgExtension>,
}

#[cw_serde]
pub struct LoanCodeIds {
    pub minter_code_id: u64,
    pub factory_code_id: u64,
    pub sg721_code_id: u64,
    pub loan_code_id: u64,
}

#[cw_serde]
pub struct RaffleCodeIds {
    pub minter_code_id: u64,
    pub factory_code_id: u64,
    pub sg721_code_id: u64,
    pub raffle_code_id: u64,
    pub nois_code_id: u64,
}

#[cw_serde]
pub struct RaffleContracts {
    pub factory: Addr,
    pub raffle: Addr,
    pub nois: Addr,
    pub cw721: Option<Addr>,
}

#[cw_serde]
pub struct MinterCodeIds {
    pub minter_code_id: u64,
    pub factory_code_id: u64,
    pub sg721_code_id: u64,
}

pub struct RaffleAccounts {
    pub creator: Addr,
    pub buyer: Addr,
}
pub struct LoanAccounts {
    pub depositor: Addr,
    pub lender: Addr,
}

#[cw_serde]
pub struct MinterAccounts {
    pub creator: Addr,
    pub buyer: Addr,
}

pub struct MinterTemplateResponse<T> {
    pub collection_response_vec: Vec<MinterCollectionResponse>,
    pub router: StargazeApp,
    pub accts: T,
}

pub struct RaffleTemplateResponse<T> {
    pub raffle_response_vec: Vec<CreateRaffleResponse>,
    pub router: StargazeApp,
    pub accts: T,
}
