use cosmrs::{bank::MsgSend, proto::cosmos::bank::v1beta1::MsgSendResponse, tx::Msg};
use cosmwasm_std::{coin, coins};
use cw_orch::prelude::*;
use raffles::{msg::ExecuteMsgFns as _, state::RaffleOptionsMsg};
use scripts::{raffles::Raffles, ELGAFAR_1};
use utils::state::{AssetInfo, Sg721Token};
pub const TEST_NFT_ADDRESS: &str =
    "stars1vvl9sevue9kqvvtnu90drtwkhflxg5lzmujmjywz7h0mz474px0swhxgz2";
pub const TOKEN_ID: &str = "1244";

pub const NOIS_TOKEN: &str = "ibc/ACCAF790E082E772691A20B0208FB972AD3A01C2DE0D7E8C479CCABF6C9F39B1";

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

    // We send some NOIS funds to the contract to register raffle randomness
    chain.commit_any::<MsgSendResponse>(
        vec![MsgSend {
            from_address: chain.sender().to_string().parse().unwrap(),
            to_address: raffles.address()?.to_string().parse().unwrap(),
            amount: vec![cosmrs::Coin {
                amount: 10000000,
                denom: NOIS_TOKEN.to_string().parse().unwrap(),
            }],
        }
        .to_any()
        .unwrap()],
        None,
    )?;

    // We create one raffle with 1 NFT and see if nois agrees to send us the randomness
    raffles.create_raffle(
        vec![AssetInfo::Sg721Token(Sg721Token {
            address: TEST_NFT_ADDRESS.to_string(),
            token_id: TOKEN_ID.to_string(),
        })],
        RaffleOptionsMsg {
            raffle_start_timestamp: None,
            raffle_duration: Some(1200), // For 20 minutes, so we can see early return of randomness
            comment: None,
            max_ticket_number: Some(5),
            max_ticket_per_address: None,
            raffle_preview: None,
            min_ticket_number: None,
            one_winner_per_asset: false,
            whitelist: None,
            gating_raffle: vec![],
        },
        AssetInfo::Coin(coin(123, "ustars")),
        None,
        &coins(45, "ustars"),
    )?;

    Ok(())
}
