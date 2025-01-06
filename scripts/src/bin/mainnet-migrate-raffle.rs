use cosmwasm_std::{to_json_binary, WasmMsg};
use cw_orch::{contract::interface_traits::CwOrchUpload, prelude::*};
use dao_cw_orch::DaoPreProposeSingle;
use dao_pre_propose_base::msg::ExecuteMsgFns;
use raffles::msg::DrandConfig;
use raffles::{msg::MigrateMsg, Raffles};
use randomness_verifier::Verifier;
use rustc_serialize::hex::FromHex;
use scripts::STARGAZE_1;

const MULTISIG_ADDRESS: &str = "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v";

pub const HEX_PUBKEY: &str = "868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31";
pub const DRAND_URL: &str = "https://api.drand.sh/8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce/public/latest";
/// One Hour
pub const DRAND_TIMEOUT: u64 = 3600u64;

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let mut chain = Daemon::builder(STARGAZE_1).build()?;
    chain.authz_granter(MULTISIG_ADDRESS);

    let verifier = Verifier::new(chain.clone());
    // verifier.upload()?;
    let raffles = Raffles::new(chain.clone());
    raffles.upload()?;
    drop(chain);

    let chain = Daemon::builder(STARGAZE_1).build()?;
    let verifier = Verifier::new(chain.clone());

    // verifier.instantiate(&Empty {}, None, None)?;
    // Then we do the migration proposal (no authz_granter this time)

    let proposal_title = "Migrate Raffles to 0.9.2";
    let proposal_description = "This migrates the raffle contract to add on_behalf_of";
    let msg = WasmMsg::Migrate {
        contract_addr: raffles.address()?.to_string(),
        new_code_id: raffles.code_id()?,
        msg: to_json_binary(&MigrateMsg {})?,
    };

    let dao_proposal = DaoPreProposeSingle::new("atlas-dao-pre-proposal", chain.clone());
    // New version is not compatible, use the old version of dao-dao and add cw-orch
    // This commit for instance : 8c945acdb0746ec84d15cfebeadcfe32122f85a2
    dao_proposal.propose(dao_pre_propose_single::contract::ProposeMessage::Propose {
        title: proposal_title.to_string(),
        description: proposal_description.to_string(),
        msgs: vec![msg.into()],
    })?;
    Ok(())
}
