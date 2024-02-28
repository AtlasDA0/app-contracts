use cw_multi_test::{Contract, ContractWrapper};
use sg_multi_test::StargazeApp;
use sg_std::StargazeMsgWrapper;

pub fn custom_mock_app() -> StargazeApp {
    StargazeApp::default()
}

pub fn contract_raffles() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        raffles::contract::execute,
        raffles::contract::instantiate,
        raffles::contract::query,
    )
    .with_sudo(raffles::contract::sudo);
    Box::new(contract)
}

pub fn contract_vending_factory() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        vending_factory::contract::execute,
        vending_factory::contract::instantiate,
        vending_factory::contract::query,
    )
    .with_sudo(vending_factory::contract::sudo);
    Box::new(contract)
}

pub fn contract_vending_minter() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        vending_minter::contract::execute,
        vending_minter::contract::instantiate,
        vending_minter::contract::query,
    )
    .with_reply(vending_minter::contract::reply);
    Box::new(contract)
}

pub fn contract_sg721_base() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg721_base::entry::execute,
        sg721_base::entry::instantiate,
        sg721_base::entry::query,
    );
    Box::new(contract)
}

pub fn contract_nft_loans() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        nft_loans_nc::contract::execute,
        nft_loans_nc::contract::instantiate,
        nft_loans_nc::contract::query,
    )
    .with_sudo(nft_loans_nc::contract::sudo);
    Box::new(contract)
}