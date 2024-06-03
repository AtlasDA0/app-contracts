use std::io::Read;

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

pub const RAFFLE_ID: u64 = 230;
pub const TOKEN_ID: &str = "239";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = CloneTesting::new(&RUNTIME, STARGAZE_1)?;

    let raffles = Raffles::new(chain.clone());

    let mut file = std::fs::File::open(Raffles::<CloneTesting>::wasm(&STARGAZE_1.into()).path())?;
    let mut wasm = Vec::<u8>::new();
    file.read_to_end(&mut wasm)?;

    let new_code_id = chain
        .app
        .borrow_mut()
        .store_wasm_code(WasmContract::Local(LocalWasmContract { code: wasm }));

    raffles
        .call_as(&Addr::unchecked(
            "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v",
        ))
        .migrate(
            &MigrateMsg {
                fee_discounts: raffles
                    .config()?
                    .fee_discounts
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            },
            new_code_id,
        )?;

    raffles.claim_raffle(RAFFLE_ID)?;

    let raffle_options = raffles.raffle_info(RAFFLE_ID)?;

    Ok(())
}
