use cosmwasm_std::Addr;
use cw_orch::daemon::Daemon;
use cw_orch::prelude::QueryHandler;
use cw_orch::prelude::TxHandler;
use raffles::msg::QueryMsgFns;
use scripts::raffles::Raffles;
use scripts::STARGAZE_1;

pub const RAFFLE_ID: u64 = 193;
pub const TOKEN_ID: &str = "239";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder(STARGAZE_1).build()?;

    let raffles = Raffles::new(chain.clone());

    let raffle_options = raffles.raffle_info(RAFFLE_ID)?;

    Ok(())
}
