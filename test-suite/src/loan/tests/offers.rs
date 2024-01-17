


// good refuse offer
// let good_refuse_offer = app.
// execute_contract(
//     Addr::unchecked(OWNER_ADDR),
//     nft_loan_addr.clone(),
//     &ExecuteMsg::RefuseOffer {
//         global_offer_id: 1.to_string(),
//     },
//     &[],
// ).unwrap();
// println!("{:#?}", good_refuse_offer);

// bad refuse offer
// let bad_refuse_offer = app
//     .execute_contract(
//         Addr::unchecked("not-owner"),
//         nft_loan_addr.clone(),
//         &ExecuteMsg::RefuseOffer {
//             global_offer_id: 1.to_string(),
//         },
//         &[],
//     )
//     .unwrap_err();
// println!("{:#?}", bad_refuse_offer);

// withdraw refused offer
// let good_withdraw_refused_offer = app
// .execute_contract(
//     Addr::unchecked(OFFERER_ADDR.to_string()),
//     nft_loan_addr.clone(),
//     &ExecuteMsg::WithdrawRefusedOffer {
//         global_offer_id: 1.to_string()
//     },
//     &[]
// ).unwrap();
// println!("{:#?}", good_withdraw_refused_offer);

// withdraw refused offer
// let bad_withdraw_refused_offer = app
// .execute_contract(
//     Addr::unchecked("not-offerer".to_string()),
//     nft_loan_addr.clone(),
//     &ExecuteMsg::WithdrawRefusedOffer {
//         global_offer_id: 1.to_string()
//     },
//     &[]
// ).unwrap_err();
// println!("{:#?}", bad_withdraw_refused_offer);

// good cancel offer
// let good_cancel_offer = app
//     .execute_contract(
//         Addr::unchecked(OFFERER_ADDR),
//         nft_loan_addr.clone(),
//         &ExecuteMsg::CancelOffer {
//             global_offer_id: 1.to_string(),
//         },
//         &[],
//     )
//     .unwrap();
// println!("{:#?}", good_cancel_offer);
