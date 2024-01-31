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


fn generate_bytes_string(num_bytes: usize, byte_value: u8) -> String {
    // Create a vector of bytes with a specific length and all bytes set to a specific value
    let bytes: Vec<u8> = vec![byte_value; num_bytes];

    // Convert the byte vector to a UTF-8 string
    String::from_utf8_lossy(&bytes).to_string()
}