use std::collections::HashSet;
use std::io::Read;

use cosmwasm_std::coins;
use cosmwasm_std::Addr;
use cw_orch::daemon::Daemon;
use cw_orch::daemon::RUNTIME;
use cw_orch::prelude::QueryHandler;
use cw_orch::prelude::TxHandler;
use cw_orch::prelude::*;
use cw_orch_clone_testing::cw_multi_test::wasm_emulation::contract::{
    LocalWasmContract, WasmContract,
};
use cw_orch_clone_testing::CloneTesting;
use raffles::msg::ExecuteMsgFns;
use raffles::msg::MigrateMsg;
use raffles::msg::QueryMsgFns;
use scripts::raffles::Raffles;
use scripts::STARGAZE_1;

pub const RAFFLE_ID: u64 = 272;

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = CloneTesting::new(&RUNTIME, STARGAZE_1)?;

    let raffles = Raffles::new(chain.clone());

    let config = raffles.config()?;

    // We try to update the randomness manually
    raffles.update_randomness(RAFFLE_ID)?;

    Ok(())
}
