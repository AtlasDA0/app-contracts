use std::vec;

use super::{
    app::StargazeApp,
    helpers::setup_block_time,
    msg::{RaffleCodeIds, RaffleContracts},
    setup_accounts_and_block::setup_accounts,
    setup_minter::common::constants::{
        CREATION_FEE_AMNT_NATIVE, CREATION_FEE_AMNT_STARS, TREASURY_ADDR,
    },
};
use crate::common_setup::{
    contract_boxes::{
        contract_raffles, contract_randomness_verifier, contract_sg721_base,
        contract_vending_factory, contract_vending_minter, custom_mock_app,
    },
    setup_minter::common::constants::{OWNER_ADDR, RAFFLE_NAME},
};
use cosmwasm_std::{coin, Addr, Binary, Coin, Decimal, Empty, Uint128};
use cw_multi_test::Executor;
use raffles::msg::{DrandConfig, InstantiateMsg};
use rustc_serialize::hex::FromHex;
use sg_std::NATIVE_DENOM;
use vending_factory::state::{ParamsExtension, VendingMinterParams};

pub const HEX_PUBKEY: &str = "868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31";
pub const DRAND_URL: &str = "https://api.drand.sh/8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce/public/latest";
/// One Hour
pub const DRAND_TIMEOUT: u64 = 3600u64;

pub fn proper_raffle_instantiate() -> (StargazeApp, RaffleContracts) {
    proper_raffle_instantiate_precise(None)
}

pub fn proper_raffle_instantiate_precise(
    max_ticket_number: Option<u32>,
) -> (StargazeApp, RaffleContracts) {
    let mut app = custom_mock_app();
    let chainid = app.block_info().chain_id.clone();
    setup_block_time(&mut app, 1647032400000000000, Some(10000), &chainid);
    setup_accounts(&mut app);
    let owner = Addr::unchecked(OWNER_ADDR);

    let code_ids = raffle_template_code_ids(&mut app);

    // TODO: setup_factory_template
    let factory_addr = app
        .instantiate_contract(
            code_ids.factory_code_id,
            owner.clone(),
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
            Some(owner.to_string()),
        )
        .unwrap();

    let randomness_verifier_addr = app
        .instantiate_contract(
            code_ids.randomness_code_id,
            owner.clone(),
            &Empty {},
            &[],
            "randomness",
            Some(owner.to_string()),
        )
        .unwrap();

    // create raffle contract
    let raffle_contract_addr = app
        .instantiate_contract(
            code_ids.raffle_code_id,
            owner.clone(),
            &InstantiateMsg {
                name: RAFFLE_NAME.to_string(),
                owner: Some(OWNER_ADDR.to_string()),
                fee_addr: Some(TREASURY_ADDR.to_owned()),
                minimum_raffle_duration: None,
                max_ticket_number,
                raffle_fee: Decimal::percent(50),
                creation_coins: vec![
                    coin(CREATION_FEE_AMNT_NATIVE, NATIVE_DENOM.to_string()),
                    coin(CREATION_FEE_AMNT_STARS, "ustars".to_string()),
                ]
                .into(),
                fee_discounts: vec![],
                drand_config: default_drand_config(&randomness_verifier_addr),
            },
            &[],
            "raffle",
            Some(owner.to_string()),
        )
        .unwrap();

    (
        app,
        RaffleContracts {
            factory: factory_addr,
            raffle: raffle_contract_addr,
            randomness_verifier: randomness_verifier_addr,
        },
    )
}

pub fn raffle_template_code_ids(router: &mut StargazeApp) -> RaffleCodeIds {
    let raffle_code_id = router.store_code(contract_raffles());
    let factory_code_id = router.store_code(contract_vending_factory());
    let minter_code_id = router.store_code(contract_vending_minter());
    let sg721_code_id = router.store_code(contract_sg721_base());
    let randomness_code_id = router.store_code(contract_randomness_verifier());

    RaffleCodeIds {
        raffle_code_id,
        minter_code_id,
        factory_code_id,
        sg721_code_id,
        randomness_code_id,
    }
}

pub fn default_drand_config(verifier: &Addr) -> DrandConfig {
    DrandConfig {
        random_pubkey: Binary::from(HEX_PUBKEY.from_hex().unwrap()),
        drand_url: DRAND_URL.to_string(),
        verify_signature_contract: verifier.clone(),
        // ONE HOUR TIMEOUT
        timeout: DRAND_TIMEOUT,
    }
}
