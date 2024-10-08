use cosmwasm_std::Addr;
use cw_orch::daemon::Daemon;
use cw_orch::prelude::QueryHandler;
use cw_orch::prelude::TxHandler;
use raffles::msg::QueryMsgFns;
use scripts::raffles::Raffles;
use scripts::STARGAZE_1;

pub const RAFFLE_ID: u64 = 245;
pub const TEST_NFT_ADDRESS: &str =
    "stars1hdjf80zg44lc65a3wqhgt62vdnpj5kpzng5dq8txar2shmgevvhst8q5me";
pub const TOKEN_ID: &str = "4620";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder().chain(STARGAZE_1).build()?;

    let raffles = Raffles::new(chain.clone());

    let _raffle_options = raffles.raffle_info(RAFFLE_ID)?;
    let _raffle_tickets = raffles.all_tickets(RAFFLE_ID, Some(50), None)?;

    // We assert the owner of the NFT is indeed the chain sender now
    let owner: cw721::OwnerOfResponse = chain.query(
        &sg721_base::msg::QueryMsg::OwnerOf {
            token_id: TOKEN_ID.to_string(),
            include_expired: None,
        },
        &Addr::unchecked(TEST_NFT_ADDRESS),
    )?;

    assert_eq!(owner.owner, chain.sender().to_string());
    Ok(())
}
