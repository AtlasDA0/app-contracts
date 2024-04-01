use crate::common_setup::app::StargazeApp;
use cosmwasm_std::{coin, coins, Addr};
use cw_multi_test::{BankSudo, SudoMsg};
use sg_std::NATIVE_DENOM;

pub const INITIAL_BALANCE: u128 = 100_000_000_000_000;

pub fn setup_accounts(
    router: &mut StargazeApp,
) -> (
    Addr,
    Addr,
    Addr,
    //  Addr, Addr, Addr, Addr, Addr
) {
    // define accounts
    let owner = Addr::unchecked("owner");
    let depositor = Addr::unchecked("depositor");
    let lender = Addr::unchecked("offerer");
    //define balances
    // fund accounts
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: owner.to_string(),
                amount: vec![
                    coin(INITIAL_BALANCE, "TOKEN"), // For staking
                    coin(INITIAL_BALANCE + 100104u128, NATIVE_DENOM.to_string()),
                    coin(INITIAL_BALANCE + 100000u128, "uflix".to_string()),
                    coin(INITIAL_BALANCE + 100000u128, "uscrt".to_string()),
                ],
            }
        }))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: depositor.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: lender.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .ok();

    (owner, depositor, lender)
}

pub fn setup_raffle_participants(router: &mut StargazeApp) -> (Addr, Addr, Addr, Addr, Addr, Addr) {
    // define accounts
    let one = Addr::unchecked("addr-one");
    let two = Addr::unchecked("addr-two");
    let three = Addr::unchecked("addr-three");
    let four = Addr::unchecked("addr-four");
    let five = Addr::unchecked("addr-five");
    let six = Addr::unchecked("addr-six");
    //define balances
    let one_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);
    let two_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);
    let three_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);
    let four_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);
    let five_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);
    let six_funds = coins(INITIAL_BALANCE, NATIVE_DENOM);

    // fund accounts
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: one.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: two.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: three.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: four.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: five.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: six.to_string(),
                amount: vec![coin(INITIAL_BALANCE, NATIVE_DENOM.to_string())],
            }
        }))
        .ok();

    // check native balances
    let one_native_balances = router.wrap().query_all_balances(one.clone()).unwrap();
    let two_native_balances = router.wrap().query_all_balances(two.clone()).unwrap();
    let three_native_balances = router.wrap().query_all_balances(three.clone()).unwrap();
    let four_native_balances = router.wrap().query_all_balances(four.clone()).unwrap();
    let five_native_balances = router.wrap().query_all_balances(five.clone()).unwrap();
    let six_native_balances = router.wrap().query_all_balances(six.clone()).unwrap();
    assert_eq!(one_native_balances, one_funds);
    assert_eq!(two_native_balances, two_funds);
    assert_eq!(three_native_balances, three_funds);
    assert_eq!(four_native_balances, four_funds);
    assert_eq!(five_native_balances, five_funds);
    assert_eq!(six_native_balances, six_funds);
    // // check native balances

    (one, two, three, four, five, six)
}
