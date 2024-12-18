use cosmwasm_std::Addr;
use cw_orch::daemon::Daemon;
use cw_orch::prelude::QueryHandler;
use cw_orch::prelude::TxHandler;
use raffles::msg::QueryMsgFns;
use raffles::Raffles;
use scripts::STARGAZE_1;

pub const RAFFLE_ID: u64 = 0;
pub const TEST_NFT_ADDRESS: &str =
    "stars1sft72uh67euvjn0tw2kyxs78rnpjgcdhd2xaevf9gatlcm2mkykqh38p9q";
pub const TOKEN_ID: &str = "365";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder(STARGAZE_1).build()?;

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

    assert_eq!(owner.owner, chain.sender_addr().to_string());
    Ok(())
}
