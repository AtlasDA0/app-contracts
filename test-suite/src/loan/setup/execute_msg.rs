use crate::common_setup::{
    contract_boxes::contract_nft_loans,
    setup_minter::common::constants::{MIN_COLLATERAL_LISTING, OWNER_ADDR, SG721_CONTRACT},
};

use super::test_msgs::{CreateLoanParams, InstantiateParams};
use anyhow::Error as anyhow_error;
use cosmwasm_std::{coin, coins, Coin, Uint128};
use cw_multi_test::{AppResponse, BankSudo, Executor, SudoMsg};
use nft_loans_nc::{
    msg::{ExecuteMsg as LoansExecuteMsg, InstantiateMsg},
    state::LoanTerms,
};
use raffles::state::ATLAS_DAO_STARGAZE_TREASURY;
use sg_std::NATIVE_DENOM;
use utils::state::{AssetInfo, Sg721Token};

pub fn instantate_loan_contract(
    params: InstantiateParams,
) -> Result<cosmwasm_std::Addr, anyhow_error> {
    let admin_account = params.admin_account;
    let funds_amount = params.funds_amount;
    let fee_rate = params.fee_rate;
    let name = params.name;

    params
        .app
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: admin_account.to_string(),
                amount: coins(funds_amount, NATIVE_DENOM),
            }
        }))
        .map_err(|err| println!("{err:?}"))
        .ok();
    let loan_code_id = params.app.store_code(contract_nft_loans());

    let msg: InstantiateMsg = InstantiateMsg {
        name,
        owner: Some(OWNER_ADDR.into()),
        treasury_addr: ATLAS_DAO_STARGAZE_TREASURY.into(),
        fee_rate,
        listing_fee_coins: vec![
            coin(MIN_COLLATERAL_LISTING, NATIVE_DENOM),
            coin(MIN_COLLATERAL_LISTING, "usstars"),
        ]
        .into(),
    };

    params.app.instantiate_contract(
        loan_code_id,
        admin_account.clone(),
        &msg,
        &coins(funds_amount, NATIVE_DENOM),
        "sg-nft-loans-nc",
        Some(admin_account.to_string()),
    )
}

pub fn create_loan_function(params: CreateLoanParams) -> Result<AppResponse, anyhow_error> {
    let owner_addr = params.owner_addr;
    let loans_contract_addr = params.loan_contract_addr;

    params.app.execute_contract(
        owner_addr.clone(),
        loans_contract_addr,
        &LoansExecuteMsg::ListCollaterals {
            tokens: vec![
                AssetInfo::Sg721Token(Sg721Token {
                    address: SG721_CONTRACT.to_string(),
                    token_id: "63".to_string(),
                }),
                AssetInfo::Sg721Token(Sg721Token {
                    address: SG721_CONTRACT.to_string(),
                    token_id: "34".to_string(),
                }),
            ],
            terms: Some(LoanTerms {
                principle: Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100),
                },
                interest: Uint128::new(50),
                duration_in_blocks: 15,
            }),
            comment: Some("be water, my friend".to_string()),
            loan_preview: None,
        },
        &[Coin {
            denom: NATIVE_DENOM.to_string(),
            amount: Uint128::new(25u128),
        }],
    )
}
