use cosmwasm_std::{to_json_binary, Binary, WasmMsg};
use cw_orch::{contract::interface_traits::CwOrchUpload, prelude::*};
use dao_cw_orch::DaoPreProposeSingle;
use dao_pre_propose_base::msg::ExecuteMsgFns;
use raffles::{
    msg::{ExecuteMsg, MigrateMsg},
    Raffles,
};
use randomness::DrandRandomness;
use scripts::STARGAZE_1;

const MULTISIG_ADDRESS: &str = "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v";
pub const RAFFLE_ID: u64 = 272;

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();

    let proposal_title = "Unlock raffle 272";
    let proposal_description = "This allows asking randomness for raffle 272 to unlock it";

    // Then we do the migration proposal (no authz_granter this time)
    let chain = Daemon::builder(STARGAZE_1).build()?;
    let raffles = Raffles::new(chain.clone());

    let msg = WasmMsg::Execute {
        contract_addr: raffles.address()?.to_string(),
        msg: to_json_binary(&ExecuteMsg::UpdateRandomness {
            raffle_id: RAFFLE_ID,
            randomness: DrandRandomness {
                round: 0,
                previous_signature: Binary::from(b"dummy"),
                signature: Binary::from(b"dummy"),
            },
        })?,
        funds: vec![],
    };

    let dao_proposal = DaoPreProposeSingle::new("atlas-dao-pre-proposal", chain.clone());
    // // New version is not compatible, use the old version of dao-dao and add cw-orch
    // // This commit for instance : 8c945acdb0746ec84d15cfebeadcfe32122f85a2
    // dao_proposal.propose(dao_pre_propose_single::contract::ProposeMessage::Propose {
    //     title: proposal_title.to_string(),
    //     description: proposal_description.to_string(),
    //     msgs: vec![msg.into()],
    // })?;
    Ok(())
}
