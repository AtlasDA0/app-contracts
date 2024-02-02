use cosmwasm_std::{coin, Addr, BlockInfo, Coin, Decimal, Timestamp, Uint128};
use cw_multi_test::{BankSudo, Executor, SudoMsg};
use nft_loans::msg::InstantiateMsg as LoanInstantiateMsg;
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;
use vending_factory::state::{ParamsExtension, VendingMinterParams};
use crate::common_setup::contract_boxes::{
    contract_nft_loans, contract_sg721_base, contract_vending_factory, contract_vending_minter,
    custom_mock_app,
};
const OWNER_ADDR: &str = "fee";
const TREASURY_ADDR: &str = "collector";
const OFFERER_ADDR: &str = "offerer";

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
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: OFFERER_ADDR.to_string(),
            amount: vec![coin(100000000000u128, NATIVE_DENOM.to_string())],
        }
    }))
    .unwrap();

    // store wasm code for nft, minter , nft-loan
    let loan_code_id = app.store_code(contract_nft_loans());
    let factory_id = app.store_code(contract_vending_factory());
    let minter_id = app.store_code(contract_vending_minter());
    let sg721_id = app.store_code(contract_sg721_base());

    // setup nft minter
    let factory_addr = app
        .instantiate_contract(
            factory_id,
            Addr::unchecked(OWNER_ADDR),
            &vending_factory::msg::InstantiateMsg {
                params: VendingMinterParams {
                    code_id: minter_id.clone(),
                    allowed_sg721_code_ids: vec![sg721_id.clone()],
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
            loan_code_id,
            Addr::unchecked(OWNER_ADDR),
            &LoanInstantiateMsg {
                name: "loan-with-insights".to_string(),
                owner: Some(Addr::unchecked(OWNER_ADDR).to_string()),
                treasury_addr: Addr::unchecked(TREASURY_ADDR).to_string(),
                fee_rate: Decimal::percent(5),
                deposit_fee_denom: vec!["usstars".to_string(), NATIVE_DENOM.to_string()],
                deposit_fee_amount: 50,
            },
            &[],
            "loans",
            Some(Addr::unchecked(OWNER_ADDR).to_string()),
        )
        .unwrap();

    (app, nft_loan_addr, factory_addr)
}
