use cosmwasm_std::to_json_binary;
use cosmwasm_std::{coin, coins, Decimal, WasmMsg};
use cw_orch::prelude::ContractInstance;
use cw_orch::prelude::TxHandler;
use cw_orch::{
    contract::interface_traits::{CwOrchInstantiate, CwOrchUpload},
    daemon::Daemon,
};
use dao_cw_orch::DaoPreProposeSingle;
use dao_pre_propose_base::msg::ExecuteMsgFns;
use p2p_trading::P2PTrading;
use raffles::msg::InstantiateMsg;
use scripts::loans::Loans;
use scripts::STARGAZE_1;
use scripts::{raffles::Raffles, ELGAFAR_1};

const MULTISIG_ADDRESS: &str = "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v";

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let chain = Daemon::builder()
        .chain(STARGAZE_1)
        .authz_granter(MULTISIG_ADDRESS)
        .build()?;

    let p2p = P2PTrading::new(chain.clone());
    p2p.upload()?;

    let proposal_title = "Instantiate P2P Trading contract";
    let proposal_description =
        "This instantiates the p2p trading contract for the first round of live testing";
    let msg = WasmMsg::Instantiate {
        admin: Some(MULTISIG_ADDRESS.to_string()),
        code_id: p2p.code_id()?,
        funds: vec![],
        label: "Atlas Dao P2P trading".to_string(),
        msg: to_json_binary(&p2p_trading_export::msg::InstantiateMsg {
            name: "AtlasDAOTrading".to_string(),
            owner: Some(MULTISIG_ADDRESS.to_string()),
            accept_trade_fee: coins(100_000_000, "ustars"),
            fund_fee: Decimal::percent(3),
            treasury: MULTISIG_ADDRESS.to_string(),
        })?,
    };

    let chain = Daemon::builder().chain(STARGAZE_1).build()?;

    // Then we do the migration proposal
    let dao_proposal = DaoPreProposeSingle::new("atlas-dao-pre-proposal", chain.clone());
    dao_proposal.propose(dao_pre_propose_single::contract::ProposeMessage::Propose {
        title: proposal_title.to_string(),
        description: proposal_description.to_string(),
        msgs: vec![msg.into()],
    })?;
    Ok(())
}
