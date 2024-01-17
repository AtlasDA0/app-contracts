// accept loan
// let good_accept_loan = app
//     .execute_contract(
//         Addr::unchecked(OFFERER_ADDR.to_string()),
//         nft_loan_addr.clone(),
//         &ExecuteMsg::AcceptLoan {
//             borrower: OWNER_ADDR.to_string(),
//             loan_id: 0,
//             comment: Some("Real living is living for others".to_string()),
//         },
//         &[Coin {
//             denom: NATIVE_DENOM.to_string(),
//             amount: Uint128::new(100),
//         }],
//     )
//     .unwrap();
// println!("{:#?}", good_accept_loan);
