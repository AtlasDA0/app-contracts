use std::vec;

use super::{
    app::StargazeApp,
    contract_boxes::contract_fake_nois,
    helpers::setup_block_time,
    msg::{RaffleCodeIds, RaffleContracts},
    nois_proxy::{self, NOIS_AMOUNT, NOIS_DENOM},
    setup_accounts_and_block::setup_accounts,
    setup_minter::common::constants::{
        CREATION_FEE_AMNT_NATIVE, CREATION_FEE_AMNT_STARS, TREASURY_ADDR,
    },
};
use crate::common_setup::{
    contract_boxes::{
        contract_raffles, contract_sg721_base, contract_vending_factory, contract_vending_minter,
        custom_mock_app,
    },
    setup_minter::common::constants::{OWNER_ADDR, RAFFLE_NAME},
};
use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use cw_multi_test::{BankSudo, Executor, SudoMsg};
use raffles::msg::InstantiateMsg;
use raffles::state::StakerFeeDiscount;
use sg_std::NATIVE_DENOM;
use vending_factory::state::{ParamsExtension, VendingMinterParams};

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

    let code_ids = raffle_template_code_ids(&mut app);

    // TODO: setup_factory_template
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

    // Create the nois contract
    let nois_addr = app
        .instantiate_contract(
            code_ids.nois_code_id,
            Addr::unchecked(OWNER_ADDR),
            &nois_proxy::InstantiateMsg {
                nois: NOIS_DENOM.to_string(),
            },
            &[],
            "nois-contract",
            None,
        )
        .unwrap();

    // create raffle contract
    let raffle_contract_addr = app
        .instantiate_contract(
            code_ids.raffle_code_id,
            Addr::unchecked(OWNER_ADDR),
            &InstantiateMsg {
                name: RAFFLE_NAME.to_string(),
                nois_proxy_addr: nois_addr.to_string(),
                nois_proxy_coin: coin(NOIS_AMOUNT, NOIS_DENOM.to_string()),
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
                atlas_dao_nft_addresses: vec![],
                staker_fee_discount: StakerFeeDiscount {
                    discount: Decimal::zero(),
                    minimum_amount: Uint128::zero(),
                },
            },
            &[],
            "raffle",
            Some(Addr::unchecked(OWNER_ADDR).to_string()),
        )
        .unwrap();

    // fund raffle contract for nois_proxy fee
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: raffle_contract_addr.clone().to_string(),
            amount: vec![coin(100000000000u128, NOIS_DENOM.to_string())],
        }
    }))
    .unwrap();

    (
        app,
        RaffleContracts {
            factory: factory_addr,
            raffle: raffle_contract_addr,
            nois: nois_addr,
        },
    )
}

pub fn raffle_template_code_ids(router: &mut StargazeApp) -> RaffleCodeIds {
    let raffle_code_id = router.store_code(contract_raffles());
    let factory_code_id = router.store_code(contract_vending_factory());
    let minter_code_id = router.store_code(contract_vending_minter());
    let sg721_code_id = router.store_code(contract_sg721_base());
    let nois_code_id = router.store_code(contract_fake_nois());

    RaffleCodeIds {
        raffle_code_id,
        minter_code_id,
        factory_code_id,
        sg721_code_id,
        nois_code_id,
    }
}
