use cosmwasm_std::{coin, coins};
use cw_orch::prelude::*;
use raffles::{msg::ExecuteMsgFns as _, state::RaffleOptionsMsg};
use scripts::{raffles::Raffles, ELGAFAR_1};
use utils::state::{AssetInfo, Sg721Token};

pub const TEST_NFT_ADDRESS: &str =
    "stars1vvl9sevue9kqvvtnu90drtwkhflxg5lzmujmjywz7h0mz474px0swhxgz2";
pub const TOKEN_ID: &str = "239";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder().chain(ELGAFAR_1).build()?;

    let raffles = Raffles::new(chain.clone());

    // We need to authorize the NFT
    chain.execute(
        &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
            spender: raffles.address()?.to_string(),
            token_id: TOKEN_ID.to_string(),
            expires: None,
        },
        &[],
        &Addr::unchecked(TEST_NFT_ADDRESS),
    )?;

    // We create one raffle with 1 NFT and see if nois agrees to send us the randomness
    raffles.create_raffle(
        vec![AssetInfo::Sg721Token(Sg721Token {
            address: TEST_NFT_ADDRESS.to_string(),
            token_id: TOKEN_ID.to_string(),
        })],
        RaffleOptionsMsg {
            raffle_start_timestamp: None,
            raffle_duration: Some(120), // For 2 minutes, so we can buy a ticket
            comment: None,
            max_ticket_number: None,
            max_ticket_per_address: None,
            raffle_preview: None,
            min_ticket_number: None,
        },
        AssetInfo::Coin(coin(123, "ustars")),
        None,
        &coins(45, "ustars"),
    )?;

    Ok(())
}
