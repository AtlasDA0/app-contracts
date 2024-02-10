use cosmwasm_std::{Addr, Coin, Decimal};

pub struct InstantiateRaffleParams<'a> {
    pub app: &'a mut sg_multi_test::StargazeApp,
    pub admin_account: Addr,
    pub funds_amount: u128,
    pub fee_rate: Decimal,
    pub name: String,
    pub nois_proxy_coin: Coin,
}
