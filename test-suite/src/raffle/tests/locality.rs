use crate::common_setup::{
    setup_accounts_and_block::setup_accounts,
    setup_minter::common::constants::OWNER_ADDR,
    setup_raffle::{proper_raffle_instantiate, raffle_template_code_ids},
};
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::Executor;
use raffles::{
    msg::ExecuteMsg,
    state::{CollectionParams, CreateLocalityParams, LocalityMinterParams},
};
use sg721::{CollectionInfo, RoyaltyInfoResponse};

#[test]
fn test_locality_creation() {
    // create testing app
    let (mut app, contracts) = proper_raffle_instantiate();
    let (owner_addr, one, two) = setup_accounts( &mut app);

    let start_time = app.block_info().time;

    // enable locality minter 
    app.execute_contract(Addr::unchecked(OWNER_ADDR), contracts.raffle.clone(), &ExecuteMsg::ToggleLocality { on: true }, &[]).unwrap();
    let good_create_locality = app.execute_contract(
        Addr::unchecked(OWNER_ADDR),
        contracts.raffle.clone(),
        &ExecuteMsg::CreateLocality {
            locality_params: CreateLocalityParams {
                init_msg: LocalityMinterParams {
                    start_time,
                    num_tokens: 10, // always = max_tickets * per_address_limit
                    mint_price: Coin::new(100, "silk"),
                    max_tickets: Some(10u32),
                    per_address_limit: Some(5u32),
                    duration: 60u64,
                    frequency: 5u64,
                    harmonics: 3u32,
                    payment_address: Some(owner_addr.to_string()),
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
    ).unwrap();
}
