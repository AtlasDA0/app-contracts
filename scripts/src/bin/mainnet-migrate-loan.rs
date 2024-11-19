use cosmwasm_std::{to_json_binary, WasmMsg};
use cw_orch::{contract::interface_traits::CwOrchUpload, prelude::*};
use dao_cw_orch::DaoPreProposeSingle;
use dao_pre_propose_base::msg::ExecuteMsgFns;
use scripts::loans::Loans;
use scripts::STARGAZE_1;

const MULTISIG_ADDRESS: &str = "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let mut chain = Daemon::builder(STARGAZE_1).build()?;
    chain.authz_granter(MULTISIG_ADDRESS);

    let loans = Loans::new(chain.clone());
    // loans.upload()?;

    let proposal_title = "Migrate Loans to 0.7.0";
    let proposal_description = "This migrates the loans contract to introduce collection offers";
    let msg = WasmMsg::Migrate {
        contract_addr: loans.address()?.to_string(),
        new_code_id: loans.code_id()?,
        msg: to_json_binary(&Empty {})?,
    };

    let chain = Daemon::builder(STARGAZE_1).build()?;

    let contract_info = chain.wasm_querier().contract_info(loans.address()?)?;
    println!("{:?}", contract_info);

    // Then we do the migration proposal
    let dao_proposal = DaoPreProposeSingle::new("atlas-dao-pre-proposal", chain.clone());
    // dao_proposal.propose(dao_pre_propose_single::contract::ProposeMessage::Propose {
    //     title: proposal_title.to_string(),
    //     description: proposal_description.to_string(),
    //     msgs: vec![msg.into()],
    // })?;

    Ok(())
}
