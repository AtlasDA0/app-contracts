use cosmwasm_std::Addr;
use cw_orch::daemon::Daemon;
use cw_orch::prelude::QueryHandler;
use cw_orch::prelude::TxHandler;
use raffles::msg::QueryMsgFns;
use scripts::{raffles::Raffles, ELGAFAR_1};

pub const RAFFLE_ID: u64 = 2;
pub const TEST_NFT_ADDRESS: &str =
    "stars1vvl9sevue9kqvvtnu90drtwkhflxg5lzmujmjywz7h0mz474px0swhxgz2";
pub const TOKEN_ID: &str = "239";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder().chain(ELGAFAR_1).build()?;

    let raffles = Raffles::new(chain.clone());

    let _raffle_options = raffles.raffle_info(RAFFLE_ID)?;

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
