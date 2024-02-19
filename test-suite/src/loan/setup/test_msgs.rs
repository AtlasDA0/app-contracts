use cosmwasm_std::{Addr, Decimal};

pub struct InstantiateParams<'a> {
    pub app: &'a mut sg_multi_test::StargazeApp,
    pub name: String,
    pub funds_amount: u128,
    pub admin_account: Addr,
    pub fee_rate: Decimal,
}


pub struct CreateLoanParams<'a> {
    pub app: &'a mut sg_multi_test::StargazeApp,
    pub loan_contract_addr: Addr,
    pub owner_addr: Addr,
    
}