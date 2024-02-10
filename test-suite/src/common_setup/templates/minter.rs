use crate::common_setup::contract_boxes::custom_mock_app;
use crate::common_setup::msg::{MinterAccounts, MinterTemplateResponse};
use crate::common_setup::setup_loan::loan_template_code_ids;
use crate::common_setup::{
    msg::MinterCollectionResponse,
    setup_accounts_and_block::setup_accounts,
    setup_minter::common::minter_params::minter_params_token,
    setup_minter::vending_minter::setup::{configure_minter, vending_minter_code_ids},
};

use cosmwasm_std::{coin, Timestamp};
use sg2::tests::{mock_collection_params_1, mock_collection_two};

use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};

pub fn loan_minter_template(num_tokens: u32) -> MinterTemplateResponse<MinterAccounts> {
    let mut app = custom_mock_app();
    let (owner, creator, buyer) = setup_accounts(&mut app);
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let collection_params = mock_collection_params_1(Some(start_time));
    let minter_params = minter_params_token(num_tokens);
    let code_ids = vending_minter_code_ids(&mut app);
    let minter_collection_response: Vec<MinterCollectionResponse> = configure_minter(
        &mut app,
        creator.clone(),
        vec![collection_params],
        vec![minter_params],
        code_ids,
    );
    MinterTemplateResponse {
        router: app,
        collection_response_vec: minter_collection_response,
        accts: MinterAccounts { creator, buyer },
    }

}
