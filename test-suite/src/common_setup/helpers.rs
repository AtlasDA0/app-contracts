use anyhow::Error;
use cosmwasm_std::{Timestamp, Uint128};
use cw_multi_test::AppResponse;

use super::app::StargazeApp;
use crate::common_setup::setup_minter::common::constants::TREASURY_ADDR;

pub fn setup_block_time(router: &mut StargazeApp, nanos: u64, height: Option<u64>, chain_id: &str) {
    let mut block = router.block_info();
    block.time = Timestamp::from_nanos(nanos);
    if let Some(h) = height {
        block.height = h;
    }
    block.chain_id = chain_id.to_string();
    router.set_block(block);
}

pub fn plus_block_seconds(router: &mut StargazeApp, secs: u64) {
    let current_time = router.block_info().time;
    let current_block = router.block_info().height;
    let chainid = router.block_info().chain_id.clone();

    setup_block_time(
        router,
        current_time.clone().plus_seconds(secs).nanos(),
        Some(current_block + secs / 10),
        &chainid.clone(),
    );
}

pub fn assert_error(res: Result<AppResponse, Error>, expected: String) {
    assert_eq!(res.unwrap_err().source().unwrap().to_string(), expected);
}

// generates long strings to check overflow
pub fn generate_bytes_string(num_bytes: usize, byte_value: u8) -> String {
    // Create a vector of bytes with a specific length and all bytes set to a specific value
    let bytes: Vec<u8> = vec![byte_value; num_bytes];
    // Convert the byte vector to a UTF-8 string
    String::from_utf8_lossy(&bytes).to_string()
}

pub fn assert_treasury_balance(app: &StargazeApp, denom: &str, amount: u128) {
    let treasury_balance = app.wrap().query_balance(TREASURY_ADDR, denom.to_string());
    assert_eq!(treasury_balance.unwrap().amount, Uint128::new(amount));
}
