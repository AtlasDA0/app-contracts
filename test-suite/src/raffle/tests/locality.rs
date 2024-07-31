use crate::common_setup::{
    app::StargazeApp, setup_accounts_and_block::setup_accounts,
    setup_minter::common::constants::OWNER_ADDR, setup_raffle::proper_raffle_instantiate,
};
use cosmwasm_std::{Addr, BlockInfo, Coin};
use cw_multi_test::Executor;
use raffles::{
    msg::{ExecuteMsg, QueryMsg},
    state::{CollectionParams, CreateLocalityParams, LocalityMinterParams},
};
use sg721::CollectionInfo;
use utils::state::AssetInfo;

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

    /// check first phase alignment mint works as expected
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
    // verify
    assert!(res);
    // more ticket purchasers

    // check mint while not in phase alignment

    // check mint during second phase alignment
}
