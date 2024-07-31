use crate::raffle::tests::locality::nois_proxy::TEST_NOIS_PREFIX;
use crate::{
    common_setup::{
        app::StargazeApp,
        nois_proxy::{self, NOIS_AMOUNT, NOIS_DENOM},
        setup_accounts_and_block::setup_accounts,
        setup_minter::common::constants::{
            CREATION_FEE_AMNT_NATIVE, CREATION_FEE_AMNT_STARS, OWNER_ADDR, RAFFLE_NAME,
            TREASURY_ADDR,
        },
        setup_raffle::proper_raffle_instantiate,
    },
    raffle::setup::helpers::send_nois_ibc_message,
};
use abstract_cw_multi_test::Contract;
use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Coin, HexBinary, Timestamp};
use cw721::{Cw721Query, Cw721QueryMsg, TokensResponse};
use cw_multi_test::{Executor, SudoMsg, WasmSudo};
use cw_orch::{mock::MockBech32, prelude::*};
use nois::NoisCallback;
use raffles::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{CollectionParams, CreateLocalityParams, LocalityMinterParams},
};
use scripts::raffles::Raffles;
use sg721::CollectionInfo;
use sg721_base::QueryMsg as Sg721QueryMsg;
use utils::state::{AssetInfo, RaffleSudoMsg, NATIVE_DENOM};

#[test]
fn test_locality_creation() {
    // create testing app
    let (mut app, contracts) = proper_raffle_instantiate();
    let (owner_addr, one, two) = setup_accounts(&mut app);
    let raffles = contracts.raffle.clone();

    let start_time = app.block_info().time;

    // enable locality minter
    app.execute_contract(
        Addr::unchecked(OWNER_ADDR),
        contracts.raffle.clone(),
        &ExecuteMsg::ToggleLocality { on: true },
        &[],
    )
    .unwrap();
    let good_create_locality = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            contracts.raffle.clone(),
            &ExecuteMsg::CreateLocality {
                locality_params: CreateLocalityParams {
                    init_msg: LocalityMinterParams {
                        start_time,
                        num_tokens: 10,
                        mint_price: Coin::new(10, "ustars"),
                        per_address_limit: Some(5u32),
                        duration: 60u64,
                        frequency: 5u64,
                        harmonics: 3u32,
                        payment_address: Some(owner_addr.to_string()),
                        ticket_limit: false,
                    },
                    collection_params: CollectionParams {
                        code_id: 4,
                        name: "infusions".into(),
                        symbol: "INFUSES".into(),
                        info: CollectionInfo {
                            creator: owner_addr.to_string(),
                            description: "infused collection, all natural".into(),
                            image: "https://image.url".into(),
                            external_link: None,
                            explicit_content: None,
                            start_trading_time: None,
                            royalty_info: None,
                        },
                    },
                },
            },
            &[],
        )
        .unwrap();

    // purchase locality tickets
    let res = app
        .execute_contract(
            one.clone(),
            raffles.clone(),
            &ExecuteMsg::BuyLocalityTicket {
                id: 0u64,
                ticket_count: 5u32,
                assets: AssetInfo::Coin(Coin::new(50, "ustars")),
            },
            &[Coin::new(50, "ustars".to_string())],
        )
        .unwrap();

    // check first phase alignment mint works as expected
    // move forward in time
    let current_time = app.block_info().time;
    let current_block = app.block_info().height;
    let chainid = app.block_info().chain_id.clone();
    app.set_block(BlockInfo {
        height: current_block + 5,
        time: current_time.clone().plus_seconds(6 * 5),
        chain_id: chainid.clone(),
    });

    // assert we are now in phase
    let res: bool = app
        .wrap()
        .query_wasm_smart(raffles.clone(), &QueryMsg::InPhase { locality: 0u64 })
        .unwrap();
    assert!(res);

    let time = app.block_info().time.minus_seconds(1);
    // run sudo stuff to simulate cron job
    // We send a nois message (that is automatic in the real world)
    let response = app
        .execute_contract(
            contracts.nois.clone(),
            contracts.raffle.clone(),
            &ExecuteMsg::NoisReceive {
                callback: NoisCallback {
                    job_id: format!("locality-0"),
                    published: time,
                    randomness: HexBinary::from_hex(
                        "0b86cdbf6bfaf5ecb2fcfe1c042ecebe2d324f7a359d03ba4a7a5230d47ed40e",
                    )
                    .unwrap(),
                },
            },
            &[],
        )
        .unwrap();

    app.sudo(SudoMsg::Wasm(WasmSudo {
        contract_addr: raffles.clone(),
        msg: to_json_binary(&RaffleSudoMsg::BeginBlock {}).unwrap(),
    }))
    .unwrap();

    // assert not in phase anymore
    let current_time = app.block_info().time;
    let current_block = app.block_info().height;
    app.set_block(BlockInfo {
        height: current_block + 1,
        time: current_time.clone().plus_seconds(6),
        chain_id: chainid.clone(),
    });
    let res: bool = app
        .wrap()
        .query_wasm_smart(raffles.clone(), &QueryMsg::InPhase { locality: 0u64 })
        .unwrap();
    assert!(!res);

    // retrieve locality collection address
    let locality_collection: Option<Addr> = app
        .wrap()
        .query_wasm_smart(
            raffles.clone(),
            &QueryMsg::LocalityCollection { locality: 0u64 },
        )
        .unwrap();

    println!("{:#?}", locality_collection);

    // verify harmonics by confirming 3 tokens are minted to the 1 participant
    let res: TokensResponse = app
        .wrap()
        .query_wasm_smart(
            locality_collection.unwrap(),
            &Sg721QueryMsg::AllTokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    println!("{:#?}", res.tokens.len());

    // more ticket purchasers

    // check mint while not in phase alignment

    // check mint during second phase alignment
}
