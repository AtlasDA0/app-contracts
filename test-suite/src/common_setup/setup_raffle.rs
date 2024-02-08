use std::vec;

use cosmwasm_std::{coin, coins, Addr, Coin, Uint128};
use cw_multi_test::Executor;

use sg_raffles::msg::InstantiateMsg;
use sg_raffles::state::NOIS_AMOUNT;
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;
use vending_factory::state::{ParamsExtension, VendingMinterParams};

use crate::common_setup::{
    contract_boxes::{
        contract_raffles, contract_sg721_base, contract_vending_factory, contract_vending_minter,
        custom_mock_app,
    },
    setup_minter::common::constants::{NOIS_PROXY_ADDR, RAFFLE_NAME},
};

use super::helpers::setup_block_time;

const OWNER_ADDR: &str = "fee";

pub fn proper_instantiate() -> (StargazeApp, Addr, Addr) {
    let mut app = custom_mock_app();
    let chainid = app.block_info().chain_id.clone();
    setup_block_time(&mut app, 1647032400000000000, Some(10000), &chainid);

    let raffle_code_id = app.store_code(contract_raffles());
    let factory_id = app.store_code(contract_vending_factory());
    let minter_id = app.store_code(contract_vending_minter());
    let sg721_id = app.store_code(contract_sg721_base());

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
                        denom: "ustars".to_string(),
                        amount: Uint128::new(100000u128),
                    },
                    min_mint_price: Coin {
                        denom: "ustars".to_string(),
                        amount: Uint128::new(100000u128),
                    },
                    mint_fee_bps: 10,
                    max_trading_offset_secs: 0,
                    extension: ParamsExtension {
                        max_token_limit: 1000,
                        max_per_address_limit: 20,
                        airdrop_mint_price: Coin {
                            denom: "ustars".to_string(),
                            amount: Uint128::new(100000u128),
                        },
                        airdrop_mint_fee_bps: 10,
                        shuffle_fee: Coin {
                            denom: "ustars".to_string(),
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

    let raffle_contract_addr = app
        .instantiate_contract(
            raffle_code_id,
            Addr::unchecked(OWNER_ADDR),
            &InstantiateMsg {
                name: RAFFLE_NAME.to_string(),
                nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
                nois_proxy_coin: coin(NOIS_AMOUNT.into(), NATIVE_DENOM.to_string()),
                owner: Some(OWNER_ADDR.to_string()),
                fee_addr: Some(OWNER_ADDR.to_owned()),
                minimum_raffle_duration: None,
                minimum_raffle_timeout: None,
                max_participant_number: None,
                raffle_fee: None,
                creation_coins: vec![
                    coin(4, NATIVE_DENOM.to_string()),
                    coin(20, "usstars".to_string()),
                ]
                .into(),
            },
            &[],
            "raffle",
            Some(Addr::unchecked(OWNER_ADDR).to_string()),
        )
        .unwrap();

    (app, raffle_contract_addr, factory_addr)
}
