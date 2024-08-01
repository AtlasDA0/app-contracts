use cw_orch::daemon::queriers::Node;
use cw_orch::daemon::Daemon;
use cw_orch::prelude::QuerierGetter;
use scripts::raffles::Raffles;
use scripts::STARGAZE_1;

pub const RAFFLE_ID: u64 = 193;
pub const TOKEN_ID: &str = "239";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder(STARGAZE_1).build()?;

    let raffles = Raffles::new(chain.clone());

    let events: Node = chain.querier();

    Ok(())
}
