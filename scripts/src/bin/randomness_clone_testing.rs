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

    // We try to update the randomness manually

    let config = raffles.config()?;

    raffles.raffle_info(RAFFLE_ID)?;

    raffles
        .call_as(&Addr::unchecked(config.nois_proxy_addr))
        .nois_receive(nois::NoisCallback {
            job_id: format!("raffle-{}", RAFFLE_ID),
            published: cosmwasm_std::Timestamp::from_seconds(1725863994),
            randomness: cosmwasm_std::HexBinary::from_hex(
                "1560f41c081ab49920672e396fd03f227e8972f8941fbd2a5436d3c33db58deb",
            )?,
        })?;
    raffles.raffle_info(RAFFLE_ID)?;

    Ok(())
}

fn migrate(raffles: &Raffles<CloneTesting>) -> anyhow::Result<()> {
    let mut file = std::fs::File::open(Raffles::<CloneTesting>::wasm(&STARGAZE_1.into()).path())?;
    let mut wasm = Vec::<u8>::new();
    file.read_to_end(&mut wasm)?;

    let new_code_id = raffles
        .get_chain()
        .app
        .borrow_mut()
        .store_wasm_code(WasmContract::Local(LocalWasmContract { code: wasm }));

    raffles
        .call_as(&Addr::unchecked(
            "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v",
        ))
        .migrate(&MigrateMsg {}, new_code_id)?;

    Ok(())
}
