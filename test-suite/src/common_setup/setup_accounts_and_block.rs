use cosmwasm_std::{coin, coins, Addr};
use cw_multi_test::{ BankSudo, SudoMsg};
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;

pub const INITIAL_BALANCE: u128 = 100_000_000_000_000;

pub fn setup_accounts(router: &mut StargazeApp) -> (Addr, Addr, Addr, Addr, Addr, Addr, Addr) {
    // define accounts
    let owner = Addr::unchecked("fee");
    let offerer = Addr::unchecked("offerer");
    let depositor = Addr::unchecked("depositor");
    let borrower = Addr::unchecked("borrower");
    let fee_collector = Addr::unchecked("collector");
    let vending_minter = Addr::unchecked("contract2");
    let sg721_contract = Addr::unchecked("contract3");
    //define balances
    let owner_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);
    let offerer_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);
    let depositor_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);
    let borrower_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);

    // fund accounts

    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: owner.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .map_err(|err| println!("{err:?}"))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: offerer.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .map_err(|err| println!("{err:?}"))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: borrower.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .map_err(|err| println!("{err:?}"))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: depositor.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .map_err(|err| println!("{err:?}"))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: fee_collector.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .map_err(|err| println!("{err:?}"))
        .ok();

    // check native balances
    let owner_native_balances = router.wrap().query_all_balances(owner.clone()).unwrap();
    assert_eq!(owner_native_balances, owner_funds);
    // check native balances
    let offer_native_balances = router.wrap().query_all_balances(offerer.clone()).unwrap();
    assert_eq!(offer_native_balances, offerer_funds);
    // check native balances
    let depositor_native_balances = router.wrap().query_all_balances(depositor.clone()).unwrap();
    assert_eq!(depositor_native_balances, depositor_funds);
    // check native balances
    let borrower_native_balances = router.wrap().query_all_balances(borrower.clone()).unwrap();
    assert_eq!(borrower_native_balances, borrower_funds);

    // return accounts
    (
        owner,
        offerer,
        depositor,
        borrower,
        fee_collector,
        vending_minter,
        sg721_contract,
    )
}
