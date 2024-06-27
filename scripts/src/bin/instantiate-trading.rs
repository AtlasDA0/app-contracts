use cosmwasm_std::{coins, Decimal};
use cw_orch::prelude::*;
use p2p_trading_export::msg::InstantiateMsg;
use scripts::trading::Trading;
use scripts::STARGAZE_1;

const MULTISIG_ADDRESS: &str = "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder()
        .chain(STARGAZE_1)
        .authz_granter(MULTISIG_ADDRESS)
        .build()?;

    let trading = Trading::new(chain.clone());
    trading.upload()?;

    let chain = Daemon::builder().chain(STARGAZE_1).build()?;

    let trading = Trading::new(chain.clone());
    trading.instantiate(
        &InstantiateMsg {
            name: "Atlas Dao P2P trading contract".to_string(),
            owner: Some(MULTISIG_ADDRESS.to_string()),
            accept_trade_fee: coins(50_000_000, "ustars"),
            fund_fee: Decimal::percent(3),
            treasury: MULTISIG_ADDRESS.to_string(),
        },
        Some(&Addr::unchecked(MULTISIG_ADDRESS)),
        None,
    )?;

    Ok(())
}
