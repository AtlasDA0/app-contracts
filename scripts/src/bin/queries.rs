use cosmwasm_std::Addr;
use cw_orch::daemon::Daemon;
use cw_orch::prelude::DefaultQueriers;
use cw_orch::prelude::QueryHandler;
use cw_orch::prelude::TxHandler;
use raffles::msg::QueryMsgFns;
use scripts::STARGAZE_1;
use scripts::{raffles::Raffles, ELGAFAR_1};
use cw_orch::prelude::IndexResponse;

pub const RAFFLE_ID: u64 = 0;
pub const TEST_NFT_ADDRESS: &str =
    "stars1vvl9sevue9kqvvtnu90drtwkhflxg5lzmujmjywz7h0mz474px0swhxgz2";
pub const TOKEN_ID: &str = "239";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder().chain(STARGAZE_1).build()?;

    let txs = chain.rt_handle.block_on(
        chain.node_querier()._find_tx_by_events(
            vec![
                format!("wasm._contract_address='stars19g0l90sxlr64ksjj9qpqz72e8dtvw772fv6dxcvva3rqas6vryzs903x60'"),
                format!("wasm.raffle_id='45'"),
                format!("wasm.action='buy_ticket'")
            ],
             Some(1), 
             None))?.into_iter().chain(chain.rt_handle.block_on(
                chain.node_querier()._find_tx_by_events(
                    vec![
                        format!("wasm._contract_address='stars19g0l90sxlr64ksjj9qpqz72e8dtvw772fv6dxcvva3rqas6vryzs903x60'"),
                        format!("wasm.raffle_id='45'"),
                        format!("wasm.action='buy_ticket'")
                    ],
                     Some(2), 
                     None))?);

    let counts = txs.into_iter().map(|t| t.event_attr_value("wasm", "ticket_count")).collect::<Result<Vec<_>, _>>()?;

    let counts = counts.into_iter().map(|c| c.parse()).collect::<Result<Vec<u64>, _>>()?;
    

    panic!("{}, {:?}, {}", counts.len(),counts, counts.iter().sum::<u64>());

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
