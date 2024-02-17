#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, HexBinary, Timestamp, Uint128};
    use cw721::OwnerOfResponse;
    use cw_multi_test::Executor;
    use nois::NoisCallback;
    use raffles::{msg::QueryMsg as RaffleQueryMsg, state::Config};
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    #[cfg(feature = "sg")]
    use sg721_base::QueryMsg as Sg721QueryMsg;

    use raffles::{
        error::ContractError,
        msg::{ExecuteMsg, InstantiateMsg, RaffleResponse},
        state::{RaffleOptionsMsg, RaffleState, NOIS_AMOUNT},
    };

    use crate::{
        common_setup::{
            contract_boxes::custom_mock_app,
            helpers::{assert_error, setup_block_time},
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_minter::common::constants::{
                 FACTORY_ADDR, MINT_PRICE, NOIS_PROXY_ADDR, OWNER_ADDR,
                RAFFLE_NAME, RAFFLE_TAX, SG721_CONTRACT,
            },
            setup_raffle::{configure_raffle_assets, proper_raffle_instantiate},
        },
        raffle::setup::{
            execute_msg::{
                buy_tickets_template, create_raffle_function, instantate_raffle_contract,
            },
            test_msgs::{CreateRaffleParams, InstantiateRaffleParams, PurchaseTicketsParams},
        },
    };

    mod init {
        use super::*;

        #[test]
        fn test_i() {
            let mut app = custom_mock_app();
            let params = InstantiateRaffleParams {
                app: &mut app,
                admin_account: Addr::unchecked(OWNER_ADDR),
                funds_amount: MINT_PRICE,
                fee_rate: RAFFLE_TAX,
                name: RAFFLE_NAME.into(),
                nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
                nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
            };
            let instantiate = instantate_raffle_contract(params);
            assert!(
                instantiate.is_ok(),
                "There is an issue instantiating a raffle"
            );
        }

        #[test]
        fn test_i_error_high_fee_rate() {
            let mut app = custom_mock_app();
            let params = InstantiateRaffleParams {
                app: &mut app,
                admin_account: Addr::unchecked(OWNER_ADDR),
                funds_amount: MINT_PRICE,
                fee_rate: Decimal::percent(200),
                name: RAFFLE_NAME.into(),
                nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
                nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
            };
            let res = instantate_raffle_contract(params).unwrap_err();
            assert_eq!(
                res.root_cause().to_string(),
                "The fee_rate you provided is not greater than 0, or less than 1"
            )
        }

        #[test]
        fn test_i_bad_nois_proxy_addr() {
            let mut app = custom_mock_app();
            let params = InstantiateRaffleParams {
                app: &mut app,
                admin_account: Addr::unchecked(OWNER_ADDR),
                funds_amount: MINT_PRICE,
                fee_rate: Decimal::percent(200),
                name: RAFFLE_NAME.into(),
                nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
                nois_proxy_addr: "".to_string(),
            };
            let res = instantate_raffle_contract(params).unwrap_err();
            assert_error(Err(res), ContractError::InvalidProxyAddress {}.to_string())
        }

        #[test]
        fn test_i_name() {
            let mut app = custom_mock_app();
            let params = InstantiateRaffleParams {
                app: &mut app,
                admin_account: Addr::unchecked(OWNER_ADDR),
                funds_amount: MINT_PRICE,
                fee_rate: RAFFLE_TAX,
                name: "80808080808080808080808080808080808080808080808080808080808080808080808080808080808088080808080808080808080808080808080808080808080808080808080808080808080808080808080808".to_string(),
                nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
                nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
            };
            let res1 = instantate_raffle_contract(params).unwrap_err();
            let params = InstantiateRaffleParams {
                app: &mut app,
                admin_account: Addr::unchecked(OWNER_ADDR),
                funds_amount: MINT_PRICE,
                fee_rate: RAFFLE_TAX,
                name: "80".to_string(),
                nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
                nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
            };
            let res2 = instantate_raffle_contract(params).unwrap_err();
            assert_eq!(res1.root_cause().to_string(), "Invalid Name");
            assert_eq!(res2.root_cause().to_string(), "Invalid Name");
        }
    }

    mod create_raffle {
        use crate::raffle::{self, setup::execute_msg::create_raffle_setup};

        use super::*;

        #[test]
        fn test_basic_create_raffle() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr);
            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr,
                owner_addr: owner_addr,
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: Some(4),
            };
            create_raffle_setup(params);
        }

        #[test]
        fn bad_raffle_creation_fee() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_address, one, two) = setup_accounts(&mut app);
            // configure raffle assets
            configure_raffle_assets(
                &mut app,
                owner_address.clone(),
                Addr::unchecked(FACTORY_ADDR),
            );
            // create a standard raffle
            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_address.clone(),
                creation_fee: vec![],
                ticket_price: None,
            };
            let msg = create_raffle_function(params);
            // confirm owner is set
            assert!(msg.is_err(), "There should be an error on this response");

            assert_error(
                Err(msg.unwrap_err()),
                ContractError::InvalidRaffleFee {}.to_string(),
            )
        }

        #[test]
        fn test_unauthorized_raffle_cancel() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_addr.clone(), factory_addr);
            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_addr,
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: Some(4),
            };
            create_raffle_setup(params);

            let invalid_cancel_raffle = app
                .execute_contract(
                    Addr::unchecked("not-owner"),
                    raffle_addr.clone(),
                    &ExecuteMsg::CancelRaffle { raffle_id: 0 },
                    &[],
                )
                .unwrap_err();
            assert_error(
                Err(invalid_cancel_raffle),
                ContractError::Unauthorized {}.to_string(),
            );
        }

        #[test]
        fn bad_ticket_sale_no_funds_provided() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_address, one, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_address.clone(), factory_addr);
            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_address.clone(),
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: None,
            };

            create_raffle_function(params).unwrap();

            let invalid_raffle_purchase = app
                .execute_contract(
                    one.clone(),
                    raffle_addr.clone(),
                    &ExecuteMsg::BuyTicket {
                        raffle_id: 1,
                        ticket_count: 1,
                        sent_assets: AssetInfo::Coin(Coin {
                            denom: "ustars".to_string(),
                            amount: Uint128::new(69u128),
                        }),
                    },
                    &[],
                )
                .unwrap_err();
            assert_error(
                Err(invalid_raffle_purchase),
                ContractError::AssetMismatch {}.to_string(),
            );
        }

        #[test]
        fn bad_modify_raffle_unauthorized() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_address, one, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_address.clone(), factory_addr);
            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_address.clone(),
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: None,
            };
            create_raffle_function(params).unwrap();

            let invalid_modify_raffle = app
                .execute_contract(
                    Addr::unchecked("not-admin"),
                    raffle_addr.clone(),
                    &ExecuteMsg::ModifyRaffle {
                        raffle_id: 0,
                        raffle_ticket_price: None,
                        raffle_options: RaffleOptionsMsg {
                            raffle_start_timestamp: None,
                            raffle_duration: None,
                            raffle_timeout: None,
                            comment: Some("rust is dooope".to_string()),
                            max_ticket_number: None,
                            max_ticket_per_address: None,
                            raffle_preview: None,
                        },
                    },
                    &[],
                )
                .unwrap_err();
            assert_error(
                Err(invalid_modify_raffle),
                ContractError::Unauthorized {}.to_string(),
            );
        }

        #[test]
        fn general_coverage() {
            // create testing app
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let current_time = app.block_info().time.clone();
            let current_block = app.block_info().height.clone();
            let chainid = app.block_info().chain_id.clone();
            let (owner_addr, _, _) = setup_accounts(&mut app);
            let (one, two, three, four, five, _) = setup_raffle_participants(&mut app);

            // create nft minter
            println!("factory_addr: {factory_addr}");
            // configure raffle assets
            configure_raffle_assets(&mut app, owner_addr.clone(), Addr::unchecked(FACTORY_ADDR));

            let params = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_addr,
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: Some(4),
            };

            // create a raffle
            let good_create_raffle = create_raffle_setup(params);

            // buy tickets
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 16,
                funds_send: vec![coin(64, "ustars")],
            };
            buy_tickets_template(params).unwrap();

            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![two.clone()],
                raffle_id: 0,
                num_tickets: 16,
                funds_send: vec![coin(64, "ustars")],
            };
            buy_tickets_template(params).unwrap();

            let res: u32 = app
                .wrap()
                .query_wasm_smart(
                    raffle_addr.clone(),
                    &RaffleQueryMsg::TicketCount {
                        owner: one.to_string(),
                        raffle_id: 0,
                    },
                )
                .unwrap();
            assert_eq!(res, 16);

            // move forward in time
            setup_block_time(
                &mut app,
                current_time.clone().plus_seconds(130).nanos(),
                Some(current_block.clone() + 100),
                &chainid.clone(),
            );

            // simulates the response from nois_proxy
            let _good_receive_randomness = app
                .execute_contract(
                    Addr::unchecked(NOIS_PROXY_ADDR),
                    raffle_addr.clone(),
                    &ExecuteMsg::NoisReceive {
                        callback: NoisCallback {
                            job_id: "raffle-0".to_string(),
                            published: current_time.clone(),
                            randomness: HexBinary::from_hex(
                                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa115",
                            )
                            .unwrap(),
                        },
                    },
                    &[],
                )
                .unwrap();

            let res: RaffleResponse = app
                .wrap()
                .query_wasm_smart(
                    raffle_addr.clone(),
                    &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
                )
                .unwrap();

            assert!(
                res.raffle_info.unwrap().randomness.is_some(),
                "randomness should have been updated into the raffle state"
            );

            // determine the raffle winner, send tokens to winner
            let _claim_ticket = app
                .execute_contract(
                    one.clone(),
                    raffle_addr.clone(),
                    &ExecuteMsg::DetermineWinner { raffle_id: 0 },
                    &[],
                )
                .unwrap();
            let res: RaffleResponse = app
                .wrap()
                .query_wasm_smart(
                    raffle_addr.clone(),
                    &RaffleQueryMsg::RaffleInfo { raffle_id: 0 },
                )
                .unwrap();
            assert_eq!(res.raffle_state, RaffleState::Claimed);

            // confirm owner of nft is now raffle winner
            let res: OwnerOfResponse = app
                .wrap()
                .query_wasm_smart(
                    SG721_CONTRACT.to_string(),
                    &Sg721QueryMsg::OwnerOf {
                        token_id: "63".to_string(),
                        include_expired: None,
                    },
                )
                .unwrap();
            assert_eq!(res.owner, two.to_string())
        }
    }
}
