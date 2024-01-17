use anyhow::Error;
use cosmwasm_std::Timestamp;
use cw_multi_test::AppResponse;
use sg_multi_test::StargazeApp;


pub fn setup_block_time(router: &mut StargazeApp, nanos: u64, height: Option<u64>) {
    let mut block = router.block_info();
    block.time = Timestamp::from_nanos(nanos);
    if let Some(h) = height {
        block.height = h;
    }
    router.set_block(block);
}

pub fn assert_error(res: Result<AppResponse, Error>, expected: String) {
    assert_eq!(res.unwrap_err().source().unwrap().to_string(), expected);
}
