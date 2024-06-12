use cw_orch::{contract::interface_traits::CwOrchUpload, daemon::Daemon};
use scripts::raffles::Raffles;
use scripts::STARGAZE_1;

const MULTISIG_ADDRESS: &str = "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder()
        .chain(STARGAZE_1)
        .authz_granter(MULTISIG_ADDRESS)
        .build()?;

    let raffles = Raffles::new(chain.clone());

    raffles.upload()?;
    // raffles.instantiate(
    //     &InstantiateMsg {
    //         name: "Raffle Contract".to_string(),
    //         nois_proxy_addr: "stars1atcndw8yfrulzux6vg6wtw2c0u4y5wvy9423255h472f4x3gn8dq0v8j45"
    //             .to_string(),
    //         nois_proxy_coin: coin(
    //             1_000_000,
    //             "ibc/ACCAF790E082E772691A20B0208FB972AD3A01C2DE0D7E8C479CCABF6C9F39B1",
    //         ),
    //         owner: None,
    //         fee_addr: Some(chain.sender().to_string()),
    //         minimum_raffle_duration: Some(60),
    //         max_ticket_number: None,
    //         raffle_fee: Decimal::percent(10),
    //         creation_coins: Some(coins(45, "ustars")),
    //         fee_discounts: vec![],
    //     },
    //     None,
    //     None,
    // )?;

    Ok(())
}