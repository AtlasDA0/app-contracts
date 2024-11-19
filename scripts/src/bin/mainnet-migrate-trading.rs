use cosmwasm_std::{to_json_binary, WasmMsg};
use cw_orch::{contract::interface_traits::CwOrchUpload, prelude::*};
use dao_cw_orch::DaoPreProposeSingle;
use dao_pre_propose_base::msg::ExecuteMsgFns;
use p2p_trading::P2PTrading;
use p2p_trading_export::msg::MigrateMsg;
use scripts::STARGAZE_1;
const MULTISIG_ADDRESS: &str = "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();

    let mut chain = Daemon::builder(STARGAZE_1).build()?;
    chain.authz_granter(MULTISIG_ADDRESS);

    let trading = P2PTrading::new(chain.clone());
    // trading.upload()?;

    let proposal_title = "Migrate Trading to 0.8.1";
    let proposal_description =
        "This migrates the trading contract to allow for permisionless trade withdrawals";
    let msg = WasmMsg::Migrate {
        contract_addr: trading.address()?.to_string(),
        new_code_id: trading.code_id()?,
        msg: to_json_binary(&MigrateMsg {})?,
    };

    // Then we do the migration proposal (no authz_granter this time)
    let chain = Daemon::builder(STARGAZE_1).build()?;

    let dao_proposal = DaoPreProposeSingle::new("atlas-dao-pre-proposal", chain.clone());
    // // New version is not compatible, use the old version of dao-dao and add cw-orch
    // // This commit for instance : 8c945acdb0746ec84d15cfebeadcfe32122f85a2
    dao_proposal.propose(dao_pre_propose_single::contract::ProposeMessage::Propose {
        title: proposal_title.to_string(),
        description: proposal_description.to_string(),
        msgs: vec![msg.into()],
    })?;
    Ok(())
}
