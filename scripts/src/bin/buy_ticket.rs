use cosmwasm_std::{coin, coins};
use cw_orch::daemon::Daemon;
use raffles::msg::ExecuteMsgFns as _;
use scripts::{raffles::Raffles, ELGAFAR_1};
use utils::state::AssetInfo;

pub const RAFFLE_ID: u64 = 2;
pub const TICKET_NUMBER: u32 = 5;
pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder().chain(ELGAFAR_1).build()?;

    let raffles = Raffles::new(chain.clone());

    // We create one raffle with 1 NFT and see if nois agrees to send us the randomness
    raffles.buy_ticket(
        RAFFLE_ID,
        AssetInfo::Coin(coin((TICKET_NUMBER as u128) * 123, "ustars")),
        TICKET_NUMBER,
        &coins((TICKET_NUMBER as u128) * 123, "ustars"),
    )?;

    Ok(())
}
