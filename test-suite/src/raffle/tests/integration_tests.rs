#[cfg(test)]
mod tests {
    use crate::common_setup::setup_raffle::{configure_raffle_assets, proper_raffle_instantiate};
    use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, HexBinary, Timestamp, Uint128};
    use cw721::OwnerOfResponse;
    use cw_multi_test::{BankSudo, Executor, SudoMsg};
    use nois::NoisCallback;
    use raffles::msg::QueryMsg as RaffleQueryMsg;
    use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
    use utils::state::{AssetInfo, Sg721Token};

    #[cfg(feature = "sg")]
    use {
        raffles::state::Config, sg2::tests::mock_collection_params_1, sg721::CollectionInfo,
        sg721_base::QueryMsg as Sg721QueryMsg,
    };

    use raffles::{
        error::ContractError,
        msg::{ExecuteMsg, InstantiateMsg, RaffleResponse},
        state::{RaffleOptionsMsg, RaffleState, NOIS_AMOUNT},
    };

    mod init {
        use std::vec;

        use raffles::{query::query_config, state::Config};

        use crate::{
            common_setup::{
                contract_boxes::custom_mock_app,
                helpers::{assert_error, setup_block_time},
                msg::MinterCollectionResponse,
                setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
                setup_minter::common::constants::{
                    CREATION_FEE_AMNT, FACTORY_ADDR, MINT_PRICE, NOIS_PROXY_ADDR, OWNER_ADDR,
                    RAFFLE_NAME, RAFFLE_TAX, SG721_CONTRACT, VENDING_MINTER,
                },
                setup_raffle::create_raffle_setup,
            },
            raffle::setup::{
                execute_msg::{
                    buy_tickets_template, create_raffle_function, instantate_raffle_contract,
                },
                test_msgs::{CreateRaffleParams, InstantiateRaffleParams, PurchaseTicketsParams},
            },
        };

        use super::*;

        #[test]
        fn test_instantiate() {
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
            instantate_raffle_contract(params).unwrap();
        }

        #[test]
        fn test_raffle_i_too_high_fee_rate() {
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
        fn test_raffle_i_bad_nois_proxy_addr() {
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
        fn test_raffle_i_name() {
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

        #[test]
        fn test_raffle_config_query() {
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
            let raffle_addr = instantate_raffle_contract(params).unwrap();

            #[cfg(feature = "sg")]
            let query_config: Config = app
                .wrap()
                .query_wasm_smart(raffle_addr, &RaffleQueryMsg::Config {})
                .unwrap();
            // println!("{:#?}", query_config);
            assert_eq!(
                query_config,
                Config {
                    name: RAFFLE_NAME.into(),
                    owner: Addr::unchecked(OWNER_ADDR),
                    fee_addr: Addr::unchecked(OWNER_ADDR),
                    last_raffle_id: Some(0),
                    minimum_raffle_duration: 1,
                    minimum_raffle_timeout: 120,
                    maximum_participant_number: None,
                    raffle_fee: RAFFLE_TAX,
                    lock: false,
                    nois_proxy_addr: Addr::unchecked(NOIS_PROXY_ADDR),
                    nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
                    creation_coins: vec![coin(CREATION_FEE_AMNT, NATIVE_DENOM)],
                }
            )
        }

        #[test]
        fn test_raffle_contract_config_permissions_coverage() {
            let (mut app, raffle_addr, _) = proper_raffle_instantiate();
            let current_time = app.block_info().time.clone();
            // errors
            // unable to update contract config
            let error_updating_config = app
                .execute_contract(
                    Addr::unchecked("not-owner"),
                    raffle_addr.clone(),
                    &ExecuteMsg::UpdateConfig {
                        name: Some("not-owner".to_string()),
                        owner: None,
                        fee_addr: None,
                        minimum_raffle_duration: None,
                        minimum_raffle_timeout: None,
                        raffle_fee: None,
                        nois_proxy_addr: None,
                        nois_proxy_coin: None,
                        creation_coins: None,
                        maximum_participant_number: None,
                    },
                    &[],
                )
                .unwrap_err();
            // unable to lock contract
            let error_locking_contract = app
                .execute_contract(
                    Addr::unchecked("not-owner"),
                    raffle_addr.clone(),
                    &raffles::msg::ExecuteMsg::ToggleLock { lock: true },
                    &[],
                )
                .unwrap_err();
            // unable to provide randomness unless nois_proxy address is sending msg
            let error_not_proxy_providing_randomness = app
                .execute_contract(
                    Addr::unchecked("not-nois-proxy"),
                    raffle_addr.clone(),
                    &raffles::msg::ExecuteMsg::NoisReceive {
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
                .unwrap_err();

            assert_error(
                Err(error_updating_config),
                ContractError::Unauthorized {}.to_string(),
            );
            assert_error(
                Err(error_locking_contract),
                ContractError::Unauthorized {}.to_string(),
            );
            assert_error(
                Err(error_not_proxy_providing_randomness),
                ContractError::UnauthorizedReceive {}.to_string(),
            );
            let _updating_config = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    raffle_addr.clone(),
                    &ExecuteMsg::UpdateConfig {
                        name: Some("new-owner".to_string()),
                        owner: Some("new-owner".to_string()),
                        fee_addr: Some("new-owner".to_string()),
                        minimum_raffle_duration: Some(60),
                        minimum_raffle_timeout: Some(240),
                        raffle_fee: Some(Decimal::percent(99)),
                        nois_proxy_addr: Some("new-owner".to_string()),
                        nois_proxy_coin: Some(coin(NOIS_AMOUNT, NATIVE_DENOM)),
                        creation_coins: Some(vec![coin(420, "new-new")]),
                        maximum_participant_number: None,
                    },
                    &[],
                )
                .unwrap();
            // good responses
            let res: Config = app
                .wrap()
                .query_wasm_smart(raffle_addr.clone(), &RaffleQueryMsg::Config {})
                .unwrap();
            println!("{:#?}", res);
            assert_eq!(
                res,
                Config {
                    name: "new-owner".to_string(),
                    owner: Addr::unchecked("new-owner"),
                    fee_addr: Addr::unchecked("new-owner"),
                    last_raffle_id: Some(0),
                    minimum_raffle_duration: 60,
                    minimum_raffle_timeout: 240,
                    maximum_participant_number: None,
                    raffle_fee: Decimal::percent(99),
                    lock: false,
                    nois_proxy_addr: Addr::unchecked("new-owner"),
                    nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
                    creation_coins: vec![coin(420, "new-new")],
                }
            )
        }

        #[test]
        fn test_basic_create_raffle() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_address, _, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_address.clone(), factory_addr);
            create_raffle_setup(&mut app, raffle_addr, owner_address);
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
            let (owner_address, _, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_address.clone(), factory_addr);
            create_raffle_setup(&mut app, raffle_addr.clone(), owner_address);

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
        fn error_bad_nois_proxy_addr() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_address, _, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_address.clone(), factory_addr);
            create_raffle_setup(&mut app, raffle_addr.clone(), owner_address);
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
        fn bad_toggle_lock() {
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

            let invalid_toggle_lock = app
                .execute_contract(
                    Addr::unchecked("not-owner"),
                    raffle_addr.clone(),
                    &ExecuteMsg::ToggleLock { lock: true },
                    &[],
                )
                .unwrap_err();
            assert_error(
                Err(invalid_toggle_lock),
                ContractError::Unauthorized {}.to_string(),
            );
        }

        #[test]
        fn good_toggle_lock() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_address, one, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_address.clone(), factory_addr);
            let create_raffle_params: CreateRaffleParams<'_> = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_address.clone(),
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: None,
            };
            create_raffle_function(create_raffle_params.into()).unwrap();

            let invalid_toggle_lock = app
                .execute_contract(
                    owner_address.clone(),
                    raffle_addr.clone(),
                    &ExecuteMsg::ToggleLock { lock: true },
                    &[],
                )
                .unwrap();
            // confirm the state is now true
            let res: Config = app
                .wrap()
                .query_wasm_smart(raffle_addr.to_string(), &RaffleQueryMsg::Config {})
                .unwrap();
            assert_eq!(res.lock, true);

            let create_raffle_params: CreateRaffleParams<'_> = CreateRaffleParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                owner_addr: owner_address.clone(),
                creation_fee: vec![coin(4, NATIVE_DENOM)],
                ticket_price: None,
            };
            // confirm raffles cannot be made & tickets cannot be bought
            let locked_creation = create_raffle_function(create_raffle_params).unwrap_err();
            assert_error(
                Err(locked_creation),
                ContractError::ContractIsLocked {}.to_string(),
            );

            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 1,
                funds_send: vec![coin(4, "ustars")],
            };
            // simulate the puchase of tickets
            let purchase_tickets = buy_tickets_template(params).unwrap_err();

            assert_error(
                Err(purchase_tickets),
                ContractError::ContractIsLocked {}.to_string(),
            )
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
        fn good_ticket_purchase() {
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let (owner_address, one, _) = setup_accounts(&mut app);
            configure_raffle_assets(&mut app, owner_address.clone(), factory_addr);
        }

        #[test]
        fn general_coverage() {
            // create testing app
            let (mut app, raffle_addr, factory_addr) = proper_raffle_instantiate();
            let current_time = app.block_info().time.clone();
            let current_block = app.block_info().height.clone();
            let chainid = app.block_info().chain_id.clone();
            let (owner_address, _, _) = setup_accounts(&mut app);
            let (one, two, three, four, five, _) = setup_raffle_participants(&mut app);

            // create nft minter
            println!("factory_addr: {factory_addr}");
            // configure raffle assets
            configure_raffle_assets(
                &mut app,
                owner_address.clone(),
                Addr::unchecked(FACTORY_ADDR),
            );

            // create a raffle
            let good_create_raffle =
                create_raffle_setup(&mut app, raffle_addr.clone(), owner_address.clone());

            // confirm owner is set
            // error if no creation fee provided when creating raffle
            //  error if unauthorized to cancel a raffle
            // err if no nois_proxy address is provided
            // errors if no funds are sent
            // errors if unauthorized to toggle lock
            // errors if unauthorized to modify raffle

            // buy tickets
            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![one.clone()],
                raffle_id: 0,
                num_tickets: 16,
                funds_send: vec![coin(64, "ustars")],
            };
            let ticket_purchase1 = buy_tickets_template(params);

            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![two.clone()],
                raffle_id: 0,
                num_tickets: 16,
                funds_send: vec![coin(64, "ustars")],
            };
            let ticket_purchase2 = buy_tickets_template(params);

            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![three.clone()],
                raffle_id: 0,
                num_tickets: 16,
                funds_send: vec![coin(64, "ustars")],
            };
            let ticket_purchase3 = buy_tickets_template(params);

            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![four.clone()],
                raffle_id: 0,
                num_tickets: 16,
                funds_send: vec![coin(64, "ustars")],
            };
            let ticket_purchase4 = buy_tickets_template(params);

            let params = PurchaseTicketsParams {
                app: &mut app,
                raffle_contract_addr: raffle_addr.clone(),
                msg_senders: vec![five.clone()],
                raffle_id: 0,
                num_tickets: 16,
                funds_send: vec![coin(64, "ustars")],
            };
            let ticket_purchase5 = buy_tickets_template(params);

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

            // try to claim ticket before randomness is requested
            let claim_but_no_randomness_yet = app
                .execute_contract(
                    one.clone(),
                    raffle_addr.clone(),
                    &ExecuteMsg::DetermineWinner { raffle_id: 0 },
                    &[],
                )
                .unwrap_err();

            // println!("{:#?}", claim_but_no_randomness_yet);
            assert_error(
                Err(claim_but_no_randomness_yet),
                ContractError::WrongStateForClaim {
                    status: RaffleState::Closed,
                }
                .to_string(),
            );

            // ensure only nois_proxy provides randomness
            let bad_recieve_randomness = app
                .execute_contract(
                    one.clone(),
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
                .unwrap_err();
            // println!("{:#?}", bad_recieve_randomness);
            assert_error(
                Err(bad_recieve_randomness),
                ContractError::UnauthorizedReceive.to_string(),
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
