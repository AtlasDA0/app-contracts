// Clone of https://github.com/DA0-DA0/dao-contracts/tree/ef21c637de7069e84f6221094dea22872930fd9a/packages/dao-testing
// Because this is not importable

use cosmwasm_std::{to_json_binary, Addr, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::Executor;
use cw_multi_test::{Contract, ContractWrapper};
use cw_utils::Duration;
use dao_interface::state::{Admin, ModuleInstantiateInfo};
use dao_pre_propose_single as cpps;
use dao_voting::deposit::DepositRefundPolicy;
use dao_voting::deposit::UncheckedDepositInfo;
use dao_voting::pre_propose::PreProposeInfo;
use dao_voting::threshold::PercentageThreshold;
use dao_voting::threshold::Threshold::ThresholdQuorum;

use crate::common_setup::app::StargazeApp;

pub const CREATOR_ADDR: &str = "dao-creator";

pub fn instantiate_with_staked_balances_governance(
    app: &mut StargazeApp,
    initial_balances: Option<Vec<Cw20Coin>>,
) -> Addr {
    let initial_balances = initial_balances.unwrap_or_else(|| {
        vec![Cw20Coin {
            address: CREATOR_ADDR.to_string(),
            amount: Uint128::new(100_000_000),
        }]
    });

    // Collapse balances so that we can test double votes.
    let initial_balances: Vec<Cw20Coin> = {
        let mut already_seen = vec![];
        initial_balances
            .into_iter()
            .filter(|Cw20Coin { address, amount: _ }| {
                if already_seen.contains(address) {
                    false
                } else {
                    already_seen.push(address.clone());
                    true
                }
            })
            .collect()
    };

    let cw20_id = app.store_code(cw20_base_contract());
    let cw20_stake_id = app.store_code(cw20_stake_contract());
    let staked_balances_voting_id = app.store_code(cw20_staked_balances_voting_contract());
    let core_contract_id = app.store_code(dao_dao_contract());
    let governance_code_id = app.store_code(proposal_single_contract());

    let instantiate_core = dao_interface::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: false,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: staked_balances_voting_id,
            msg: to_json_binary(&dao_voting_cw20_staked::msg::InstantiateMsg {
                active_threshold: None,
                token_info: dao_voting_cw20_staked::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token.".to_string(),
                    name: "DAO DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances: initial_balances.clone(),
                    marketing: None,
                    staking_code_id: cw20_stake_id,
                    unstaking_duration: Some(Duration::Height(6)),
                    initial_dao_balance: None,
                },
            })
            .unwrap(),
            admin: None,
            funds: vec![],
            label: "DAO DAO voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: governance_code_id,
            label: "DAO DAO governance module.".to_string(),
            admin: Some(Admin::CoreModule {}),
            msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                threshold: ThresholdQuorum {
                    quorum: PercentageThreshold::Percent(Decimal::percent(15)),
                    threshold: PercentageThreshold::Majority {},
                },
                max_voting_period: Duration::Time(604800), // One week.
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: false,
                pre_propose_info: get_pre_propose_info(
                    app,
                    Some(UncheckedDepositInfo {
                        denom: dao_voting::deposit::DepositToken::VotingModuleToken {},
                        amount: Uint128::new(10_000_000),
                        refund_policy: DepositRefundPolicy::OnlyPassed,
                    }),
                    false,
                ),
                close_proposal_on_execution_failure: true,
            })
            .unwrap(),
            funds: vec![],
        }],
        initial_items: None,
    };

    let core_addr = app
        .instantiate_contract(
            core_contract_id,
            Addr::unchecked(CREATOR_ADDR),
            &instantiate_core,
            &[],
            "DAO DAO",
            None,
        )
        .unwrap();

    let gov_state: dao_interface::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(
            core_addr.clone(),
            &dao_interface::msg::QueryMsg::DumpState {},
        )
        .unwrap();
    let voting_module = gov_state.voting_module;

    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &dao_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &dao_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Stake all the initial balances.
    for Cw20Coin { address, amount } in initial_balances {
        app.execute_contract(
            Addr::unchecked(address),
            token_contract.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: staking_contract.to_string(),
                amount,
                msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();
    }

    // Update the block so that those staked balances appear.
    app.update_block(|block| block.height += 1);

    core_addr
}

pub fn proposal_single_contract() -> Box<dyn Contract<sg_std::StargazeMsgWrapper>> {
    let contract = ContractWrapper::new_with_empty(
        dao_proposal_single::contract::execute,
        dao_proposal_single::contract::instantiate,
        dao_proposal_single::contract::query,
    )
    .with_reply_empty(dao_proposal_single::contract::reply)
    .with_migrate_empty(dao_proposal_single::contract::migrate);
    Box::new(contract)
}

pub fn cw20_base_contract() -> Box<dyn Contract<sg_std::StargazeMsgWrapper>> {
    let contract = ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub fn cw20_stake_contract() -> Box<dyn Contract<sg_std::StargazeMsgWrapper>> {
    let contract = ContractWrapper::new_with_empty(
        cw20_stake::contract::execute,
        cw20_stake::contract::instantiate,
        cw20_stake::contract::query,
    );
    Box::new(contract)
}

pub fn cw20_staked_balances_voting_contract() -> Box<dyn Contract<sg_std::StargazeMsgWrapper>> {
    let contract = ContractWrapper::new_with_empty(
        dao_voting_cw20_staked::contract::execute,
        dao_voting_cw20_staked::contract::instantiate,
        dao_voting_cw20_staked::contract::query,
    )
    .with_reply_empty(dao_voting_cw20_staked::contract::reply);
    Box::new(contract)
}

pub fn dao_dao_contract() -> Box<dyn Contract<sg_std::StargazeMsgWrapper>> {
    let contract = ContractWrapper::new_with_empty(
        dao_dao_core::contract::execute,
        dao_dao_core::contract::instantiate,
        dao_dao_core::contract::query,
    )
    .with_reply_empty(dao_dao_core::contract::reply)
    .with_migrate_empty(dao_dao_core::contract::migrate);
    Box::new(contract)
}

pub fn pre_propose_single_contract() -> Box<dyn Contract<sg_std::StargazeMsgWrapper>> {
    let contract = ContractWrapper::new_with_empty(
        cpps::contract::execute,
        cpps::contract::instantiate,
        cpps::contract::query,
    );
    Box::new(contract)
}

pub(crate) fn get_pre_propose_info(
    app: &mut StargazeApp,
    deposit_info: Option<UncheckedDepositInfo>,
    open_proposal_submission: bool,
) -> PreProposeInfo {
    let pre_propose_contract = app.store_code(pre_propose_single_contract());
    PreProposeInfo::ModuleMayPropose {
        info: ModuleInstantiateInfo {
            code_id: pre_propose_contract,
            msg: to_json_binary(&cpps::InstantiateMsg {
                deposit_info,
                open_proposal_submission,
                extension: Empty::default(),
            })
            .unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "pre_propose_contract".to_string(),
        },
    }
}
