use crate::common_setup::{
    contract_boxes::contract_nft_loans,
    setup_minter::common::constants::{LOAN_NAME, MIN_COLLATERAL_LISTING, OWNER_ADDR},
};

use super::test_msgs::InstantiateParams;
use anyhow::Error as anyhow_error;
use cosmwasm_std::{coin, coins, Addr};
use cw_multi_test::{BankSudo, Executor, SudoMsg};
use nft_loans::msg::InstantiateMsg;
use raffles::state::ATLAS_DAO_STARGAZE_TREASURY;
use sg_std::NATIVE_DENOM;

pub fn instantate_contract(params: InstantiateParams) -> Result<cosmwasm_std::Addr, anyhow_error> {
    let admin_account = params.admin_account;
    let funds_amount = params.funds_amount;
    let fee_rate = params.fee_rate;

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
        name: LOAN_NAME.into(),
        owner: Some(OWNER_ADDR.into()),
        treasury_addr: ATLAS_DAO_STARGAZE_TREASURY.into(),
        fee_rate,
        listing_fee_coins: vec![
            coin(MIN_COLLATERAL_LISTING, NATIVE_DENOM),
            coin(MIN_COLLATERAL_LISTING, "usstars"),
        ].into(),
    };

    params.app.instantiate_contract(
        loan_code_id,
        Addr::unchecked(admin_account.clone()),
        &msg,
        &coins(funds_amount, NATIVE_DENOM),
        "sg-non-custodial-loans",
        Some(Addr::unchecked(admin_account).to_string()),
    )
}
