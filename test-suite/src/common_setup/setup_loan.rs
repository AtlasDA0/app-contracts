use crate::common_setup::contract_boxes::{
    contract_nft_loans, contract_sg721_base, contract_vending_factory, contract_vending_minter,
    custom_mock_app,
};
use cosmwasm_std::{coin, Addr, BlockInfo, Coin, Decimal, Timestamp, Uint128};
use cw_multi_test::{BankSudo, Executor, SudoMsg};
use nft_loans_nc::msg::InstantiateMsg as LoanInstantiateMsg;
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;
use vending_factory::state::{ParamsExtension, VendingMinterParams};

use super::{
    msg::{LoanCodeIds, LoanSetupParams},
    setup_minter::{self, common::constants::{OFFERER_ADDR, OWNER_ADDR, TREASURY_ADDR}, vending_minter::setup::setup_minter_contract},
};

pub fn loan_template_code_ids(router: &mut StargazeApp) -> LoanCodeIds {
    let minter_code_id = router.store_code(contract_vending_minter());
    println!("minter_code_id: {minter_code_id}");

    let factory_code_id = router.store_code(contract_vending_factory());
    println!("factory_code_id: {factory_code_id}");

    let sg721_code_id = router.store_code(contract_sg721_base());
    println!("sg721_code_id: {sg721_code_id}");

    let loan_code_id = router.store_code(contract_nft_loans());
    println!("loan_code_id: {loan_code_id}");

    LoanCodeIds {
        minter_code_id,
        factory_code_id,
        sg721_code_id,
        loan_code_id,
    }
}

pub fn proper_loan_instantiate() -> (StargazeApp, Addr, Addr) {
    // setup mock blockchain environment
    let mut app = custom_mock_app();
    let chainid = app.block_info().chain_id.clone();

    app.set_block(BlockInfo {
        height: 10000,
        time: Timestamp::from_nanos(1647032400000000000),
        chain_id: chainid,
    });

    // fund test account
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: OWNER_ADDR.to_string(),
            amount: vec![coin(100000000000u128, NATIVE_DENOM.to_string())],
        }
    }))
    .unwrap();
    // fund test account
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: OWNER_ADDR.to_string(),
            amount: vec![coin(100000000000u128, "uscrt".to_string())],
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: OFFERER_ADDR.to_string(),
            amount: vec![coin(100000000000u128, NATIVE_DENOM.to_string())],
        }
    }))
    .unwrap();

    // store wasm code for nft, minter , nft-loan
    let code_ids = loan_template_code_ids(&mut app);

    // setup nft minter
    let factory_addr = app
        .instantiate_contract(
            code_ids.factory_code_id,
            Addr::unchecked(OWNER_ADDR),
            &vending_factory::msg::InstantiateMsg {
                params: VendingMinterParams {
                    code_id: code_ids.minter_code_id,
                    allowed_sg721_code_ids: vec![code_ids.sg721_code_id],
                    frozen: false,
                    creation_fee: Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100000u128),
                    },
                    min_mint_price: Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100000u128),
                    },
                    mint_fee_bps: 10,
                    max_trading_offset_secs: 0,
                    extension: ParamsExtension {
                        max_token_limit: 1000,
                        max_per_address_limit: 20,
                        airdrop_mint_price: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100000u128),
                        },
                        airdrop_mint_fee_bps: 10,
                        shuffle_fee: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100000u128),
                        },
                    },
                },
            },
            &[],
            "factory",
            Some(OWNER_ADDR.to_string()),
        )
        .unwrap();
    // create nft-loan contract
    let nft_loan_addr = app
        .instantiate_contract(
            code_ids.loan_code_id,
            Addr::unchecked(OWNER_ADDR),
            &LoanInstantiateMsg {
                name: "loan-with-insights".to_string(),
                owner: Some(Addr::unchecked(OWNER_ADDR).to_string()),
                treasury_addr: Addr::unchecked(TREASURY_ADDR).to_string(),
                fee_rate: Decimal::percent(5),
                listing_fee_coins: vec![coin(55, NATIVE_DENOM.to_string()), coin(45, "usstars")]
                    .into(),
            },
            &[],
            "loans",
            Some(Addr::unchecked(OWNER_ADDR).to_string()),
        )
        .unwrap();

    (app, nft_loan_addr, factory_addr)
}
