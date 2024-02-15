use std::vec;

use super::{
    helpers::setup_block_time,
    msg::{MinterCodeIds, RaffleCodeIds},
    setup_minter::vending_minter::setup::vending_minter_code_ids,
};
use crate::common_setup::{
    contract_boxes::{
        contract_raffles, contract_sg721_base, contract_vending_factory, contract_vending_minter,
        custom_mock_app,
    },
    msg::{MinterAccounts, MinterCollectionResponse},
    setup_accounts_and_block::setup_accounts,
    setup_minter::{
        common::{
            constants::{NOIS_PROXY_ADDR, OWNER_ADDR, RAFFLE_NAME},
            minter_params::minter_params_token,
        },
        vending_minter::setup::configure_minter,
    },
    templates::raffles::raffle_minter_template,
};
use cosmwasm_std::{coin, coins, Addr, Coin, Decimal, Timestamp, Uint128};
use cw_multi_test::Executor;
use raffles::{msg::InstantiateMsg, state::NOIS_AMOUNT};
use sg2::tests::mock_collection_params_1;
use sg_multi_test::StargazeApp;
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use vending_factory::state::{ParamsExtension, VendingMinterParams};

pub fn proper_raffle_instantiate() -> (StargazeApp, Addr, Addr) {
    let mut app = custom_mock_app();
    let chainid = app.block_info().chain_id.clone();
    setup_block_time(&mut app, 1647032400000000000, Some(10000), &chainid);

    let code_ids = raffle_template_code_ids(&mut app);

    // TODO: setup_factory_template
    let factory_addr = app
        .instantiate_contract(
            code_ids.factory_code_id,
            Addr::unchecked(OWNER_ADDR),
            &vending_factory::msg::InstantiateMsg {
                params: VendingMinterParams {
                    code_id: code_ids.minter_code_id.clone(),
                    allowed_sg721_code_ids: vec![code_ids.sg721_code_id.clone()],
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
            code_ids.raffle_code_id,
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
                raffle_fee: Decimal::percent(0),
                creation_coins: vec![
                    coin(4, NATIVE_DENOM.to_string()),
                    coin(20, "u stars".to_string()),
                ]
                .into(),
            },
            &[],
            "raffle",
            Some(Addr::unchecked(OWNER_ADDR).to_string()),
        )
        .unwrap();

    // let setup = raffle_minter_template(2);
    // let res: MinterAccounts = setup.accts;

    println!("raffle_contract_addr: {raffle_contract_addr}");
    println!("factory_addr: {factory_addr}");
    // println!("{:#?}", res);

    (app, raffle_contract_addr, factory_addr)
}

pub fn configure_raffle_assets(
    app: &mut StargazeApp,
    minter_admin: Addr,
    minter_addr: Addr,
    num_nfts: u64,
) -> () {
    // VENDING_MINTER is minter
    let _mint_nft_tokens = app
        .execute_contract(
            minter_admin.clone(),
            minter_addr.clone(),
            &vending_minter::msg::ExecuteMsg::Mint {},
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100000u128),
            }],
        )
        .unwrap();
    // println!("{:#?}", _mint_nft_tokens);
}

pub fn raffle_template_code_ids(router: &mut StargazeApp) -> RaffleCodeIds {
    let raffle_code_id = router.store_code(contract_raffles());
    let factory_code_id = router.store_code(contract_vending_factory());
    let minter_code_id = router.store_code(contract_vending_minter());
    let sg721_code_id = router.store_code(contract_sg721_base());

    println!("raffle_code_id: {raffle_code_id}");
    println!("minter_code_id: {minter_code_id}");
    println!("factory_code_id: {factory_code_id}");
    println!("sg721_code_id: {sg721_code_id}");
    RaffleCodeIds {
        raffle_code_id,
        minter_code_id,
        factory_code_id,
        sg721_code_id,
    }
}
