#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, BlockInfo, Coin, Timestamp, Uint128};
    use cw_multi_test::Executor;
    use raffles::msg::InstantiateMsg;
    use sg_multi_test::StargazeApp;
    use sg_std::NATIVE_DENOM;
    use vending_factory::state::{ParamsExtension, VendingMinterParams};

    use crate::common_setup::contract_boxes::{contract_raffles, custom_mock_app};
    use crate::common_setup::contract_boxes::{
        contract_sg721_base, contract_vending_factory, contract_vending_minter,
    };

    const NOIS_PROXY_ADDR: &str = "nois";
    const NOIS_AMOUNT: u128 = 50;
    const OWNER_ADDR: &str = "fee";
    const NAME: &str = "raffle param name";
    const CREATION_FEE_AMNT: u128 = 50;
    const VENDING_MINTER: &str = "contract2";
    const SG721_CONTRACT: &str = "contract3";

    pub fn proper_instantiate() -> (StargazeApp, Addr, Addr) {
        let mut app = custom_mock_app();
        let chainid = app.block_info().chain_id.clone();
        app.set_block(BlockInfo {
            height: 10000,
            time: Timestamp::from_nanos(1647032400000000000),
            chain_id: chainid,
        });
        let raffle_code_id = app.store_code(contract_raffles());
        let factory_id = app.store_code(contract_vending_factory());
        let minter_id = app.store_code(contract_vending_minter());
        let sg721_id = app.store_code(contract_sg721_base());

        let factory_addr = app
            .instantiate_contract(
                factory_id,
                Addr::unchecked(OWNER_ADDR),
                &vending_factory::msg::InstantiateMsg {
                    params: VendingMinterParams {
                        code_id: minter_id.clone(),
                        allowed_sg721_code_ids: vec![sg721_id.clone()],
                        frozen: false,
                        creation_fee: Coin {
                            denom: "ustars".to_string(),
                            amount: Uint128::new(100000u128),
                        },
                        min_mint_price: Coin {
                            denom: "ustars".to_string(),
                            amount: Uint128::new(100000u128),
                        },
                        mint_fee_bps: 10,
                        max_trading_offset_secs: 0,
                        extension: ParamsExtension {
                            max_token_limit: 1000,
                            max_per_address_limit: 20,
                            airdrop_mint_price: Coin {
                                denom: "ustars".to_string(),
                                amount: Uint128::new(100000u128),
                            },
                            airdrop_mint_fee_bps: 10,
                            shuffle_fee: Coin {
                                denom: "ustars".to_string(),
                                amount: Uint128::new(100000u128),
                            },
                        },
                    },
                },
                &[],
                "factory",
                Some(OWNER_ADDR.to_string()),
            )
            .unwrap();

        let raffle_contract_addr = app
            .instantiate_contract(
                raffle_code_id,
                Addr::unchecked(OWNER_ADDR),
                &InstantiateMsg {
                    name: NAME.to_string(),
                    nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
                    nois_proxy_denom: NATIVE_DENOM.to_string(),
                    nois_proxy_amount: NOIS_AMOUNT.into(),
                    creation_fee_denom: Some(vec![NATIVE_DENOM.to_string(), "usstars".to_string()]),
                    creation_fee_amount: Some(CREATION_FEE_AMNT.into()),
                    owner: Some(OWNER_ADDR.to_string()),
                    fee_addr: Some(OWNER_ADDR.to_owned()),
                    minimum_raffle_duration: None,
                    minimum_raffle_timeout: None,
                    max_participant_number: None,
                    raffle_fee: None,
                },
                &[],
                "raffle",
                Some(Addr::unchecked(OWNER_ADDR).to_string()),
            )
            .unwrap();

        (app, raffle_contract_addr, factory_addr)
    }

    mod init {
        use cosmwasm_std::{coin, Coin, Empty, HexBinary, Uint128};
        use cw_multi_test::{BankSudo, SudoMsg};
        use nois::NoisCallback;
        use raffles::{
            error::ContractError,
            msg::{ExecuteMsg, RaffleResponse},
            state::{RaffleOptionsMsg, RaffleState},
        };
        use sg721::CollectionInfo;
        use utils::state::{AssetInfo, Sg721Token};
        use vending_factory::msg::VendingMinterCreateMsg;

        use crate::common_setup::helpers::assert_error;

        use super::*;

        #[test]
        fn can_init() {
            // create testing app
            let (mut app, raffle_contract_addr, factory_addr) = proper_instantiate();
            let query_config: raffles::msg::ConfigResponse = app
                .wrap()
                .query_wasm_smart(
                    raffle_contract_addr.clone(),
                    &raffles::msg::QueryMsg::Config {},
                )
                .unwrap();
            assert_eq!(query_config.owner, Addr::unchecked("fee"));

            let current_time = app.block_info().time.clone();
            let current_block = app.block_info().height.clone();
            let chainid = app.block_info().chain_id.clone();

            // fund test account
            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: OWNER_ADDR.to_string(),
                    amount: vec![coin(100000000000u128, "ustars".to_string())],
                }
            }))
            .unwrap();
            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: "wallet-1".to_string(),
                    amount: vec![coin(100000000000u128, "ustars".to_string())],
                }
            }))
            .unwrap();
            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: "wallet-2".to_string(),
                    amount: vec![coin(100000000000u128, "ustars".to_string())],
                }
            }))
            .unwrap();
            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: "wallet-3".to_string(),
                    amount: vec![coin(100000000000u128, "ustars".to_string())],
                }
            }))
            .unwrap();
            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: "wallet-4".to_string(),
                    amount: vec![coin(100000000000u128, "ustars".to_string())],
                }
            }))
            .unwrap();
            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: "wallet-5".to_string(),
                    amount: vec![coin(100000000000u128, "ustars".to_string())],
                }
            }))
            .unwrap();

            let raffle_code_id = app.store_code(contract_raffles());

            // create nft minter
            let _create_nft_minter = app.execute_contract(
                Addr::unchecked(OWNER_ADDR),
                factory_addr.clone(),
                &vending_factory::msg::ExecuteMsg::CreateMinter {
                    0: VendingMinterCreateMsg {
                        init_msg: vending_factory::msg::VendingMinterInitMsgExtension {
                            base_token_uri: "ipfs://aldkfjads".to_string(),
                            payment_address: Some(OWNER_ADDR.to_string()),
                            start_time: current_time.clone(),
                            num_tokens: 100,
                            mint_price: coin(Uint128::new(100000u128).u128(), NATIVE_DENOM),
                            per_address_limit: 3,
                            whitelist: None,
                        },
                        collection_params: sg2::msg::CollectionParams {
                            code_id: 4,
                            name: "Collection Name".to_string(),
                            symbol: "COL".to_string(),
                            info: CollectionInfo {
                                creator: "creator".to_string(),
                                description: String::from("Atlanauts"),
                                image: "https://example.com/image.png".to_string(),
                                external_link: Some(
                                    "https://example.com/external.html".to_string(),
                                ),
                                start_trading_time: None,
                                explicit_content: Some(false),
                                royalty_info: None,
                            },
                        },
                    },
                },
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100000u128),
                }],
            );
            // println!("{:#?}", create_nft_minter);

            // VENDING_MINTER is minter
            let _mint_nft_tokens = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    Addr::unchecked(VENDING_MINTER),
                    &vending_minter::msg::ExecuteMsg::Mint {},
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100000u128),
                    }],
                )
                .unwrap();
            // println!("{:#?}", _mint_nft_tokens);

            // token id 41
            let _grant_approval = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    Addr::unchecked(SG721_CONTRACT),
                    &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                        spender: raffle_contract_addr.to_string(),
                        token_id: "41".to_string(),
                        expires: None,
                    },
                    &[],
                )
                .unwrap();
            // println!("{:#?}", _grant_approval);

            // create a raffle
            let _good_create_raffle = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::CreateRaffle {
                        owner: Some(OWNER_ADDR.to_string()),
                        assets: vec![AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "41".to_string(),
                        })],
                        raffle_options: RaffleOptionsMsg {
                            raffle_start_timestamp: Some(current_time.clone()),
                            raffle_duration: None,
                            raffle_timeout: None,
                            comment: None,
                            max_participant_number: None,
                            max_ticket_per_address: None,
                            raffle_preview: None,
                        },
                        raffle_ticket_price: AssetInfo::Coin(Coin {
                            denom: "ustars".to_string(),
                            amount: Uint128::new(100u128),
                        }),
                    },
                    &[coin(50, "ustars")],
                )
                .unwrap();
            // println!("{:#?}", _good_create_raffle);

            let res: raffles::msg::RaffleResponse = app
                .wrap()
                .query_wasm_smart(
                    raffle_contract_addr.clone(),
                    &raffles::msg::QueryMsg::RaffleInfo { raffle_id: 0 },
                )
                .unwrap();
            assert_eq!(res.raffle_info.unwrap().owner, "fee");

            // no creation fee provided
            let create_raffle_no_creation_fee_error = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::CreateRaffle {
                        owner: Some(OWNER_ADDR.to_string()),
                        assets: vec![AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "41".to_string(),
                        })],
                        raffle_options: RaffleOptionsMsg {
                            raffle_start_timestamp: None,
                            raffle_duration: Some(30),
                            raffle_timeout: None,
                            comment: None,
                            max_participant_number: None,
                            max_ticket_per_address: None,
                            raffle_preview: None,
                        },
                        raffle_ticket_price: AssetInfo::Coin(Coin {
                            denom: "ustars".to_string(),
                            amount: Uint128::new(100u128),
                        }),
                    },
                    &[],
                )
                .unwrap_err();
            // println!("{:#?}", create_raffle_no_creation_fee_error);

            assert_error(
                Err(create_raffle_no_creation_fee_error),
                ContractError::InvalidRaffleFee {}.to_string(),
            );

            // invalid cancel
            let invalid_cancel_raffle = app
                .execute_contract(
                    Addr::unchecked("not-owner"),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::CancelRaffle { raffle_id: 0 },
                    &[],
                )
                .unwrap_err();
            // println!("{:#?}", invalid_cancel_raffle);
            assert_error(
                Err(invalid_cancel_raffle),
                ContractError::Unauthorized {}.to_string(),
            );

            // invalid proxy
            let invalid_proxy = app
                .instantiate_contract(
                    raffle_code_id,
                    Addr::unchecked(OWNER_ADDR),
                    &InstantiateMsg {
                        name: NAME.to_string(),
                        nois_proxy_addr: "".to_string(),
                        nois_proxy_denom: NATIVE_DENOM.to_string(),
                        nois_proxy_amount: NOIS_AMOUNT.into(),
                        creation_fee_denom: Some(vec![NATIVE_DENOM.to_string()]),
                        creation_fee_amount: Some(CREATION_FEE_AMNT.into()),
                        owner: Some(OWNER_ADDR.to_string()),
                        fee_addr: Some(OWNER_ADDR.to_owned()),
                        minimum_raffle_duration: None,
                        minimum_raffle_timeout: None,
                        max_participant_number: None,
                        raffle_fee: None,
                    },
                    &[],
                    "raffle",
                    None,
                )
                .unwrap_err();
            // println!("{:#?}", invalid_proxy);
            assert_error(
                Err(invalid_proxy),
                ContractError::InvalidProxyAddress {}.to_string(),
            );

            // invalid buy ticket
            let invalid_raffle_purchase = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::BuyTicket {
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
            // println!("{:#?}", invalid_raffle_purchase);
            assert_error(
                Err(invalid_raffle_purchase),
                ContractError::AssetMismatch {}.to_string(),
            );

            // invalid toggle lock
            let invalid_toggle_lock = app
                .execute_contract(
                    Addr::unchecked("not-owner"),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::ToggleLock { lock: true },
                    &[],
                )
                .unwrap_err();
            // println!("{:#?}", invalid_toggle_lock);
            assert_error(
                Err(invalid_toggle_lock),
                ContractError::Unauthorized {}.to_string(),
            );

            // invalid modify raffle
            let invalid_modify_raffle = app
                .execute_contract(
                    Addr::unchecked("not-admin"),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::ModifyRaffle {
                        raffle_id: 0,
                        raffle_ticket_price: None,
                        raffle_options: RaffleOptionsMsg {
                            raffle_start_timestamp: None,
                            raffle_duration: None,
                            raffle_timeout: None,
                            comment: Some("rust is dooope".to_string()),
                            max_participant_number: None,
                            max_ticket_per_address: None,
                            raffle_preview: None,
                        },
                    },
                    &[],
                )
                .unwrap_err();
            // println!("{:#?}", invalid_modify_raffle);
            assert_error(
                Err(invalid_modify_raffle),
                ContractError::Unauthorized {}.to_string(),
            );

            // buy tickets
            let _ticket_purchase1 = app
                .execute_contract(
                    Addr::unchecked("wallet-1"),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: 16,
                        sent_assets: AssetInfo::Coin(Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(1600u128),
                        }),
                    },
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1600u128),
                    }],
                )
                .unwrap();
            // println!("{:#?}", _ticket_purchase1);
            let _ticket_purchase2 = app
                .execute_contract(
                    Addr::unchecked("wallet-2"),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: 16,
                        sent_assets: AssetInfo::Coin(Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(1600u128),
                        }),
                    },
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1600u128),
                    }],
                )
                .unwrap();
            // println!("{:#?}", _ticket_purchase2);
            let _ticket_purchase3 = app
                .execute_contract(
                    Addr::unchecked("wallet-3"),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: 16,
                        sent_assets: AssetInfo::Coin(Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(1600u128),
                        }),
                    },
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1600u128),
                    }],
                )
                .unwrap();
            // println!("{:#?}", _ticket_purchase3);
            let _ticket_purchase4 = app
                .execute_contract(
                    Addr::unchecked("wallet-4"),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: 16,
                        sent_assets: AssetInfo::Coin(Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(1600u128),
                        }),
                    },
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1600u128),
                    }],
                )
                .unwrap();
            // println!("{:#?}", _ticket_purchase4);
            let _ticket_purchase5 = app
                .execute_contract(
                    Addr::unchecked("wallet-5"),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::BuyTicket {
                        raffle_id: 0,
                        ticket_count: 16,
                        sent_assets: AssetInfo::Coin(Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(1600u128),
                        }),
                    },
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1600u128),
                    }],
                )
                .unwrap();
            // println!("{:#?}", _ticket_purchase5);

            let res: u32 = app
                .wrap()
                .query_wasm_smart(
                    raffle_contract_addr.clone(),
                    &raffles::msg::QueryMsg::TicketCount {
                        owner: Addr::unchecked("wallet-1").to_string(),
                        raffle_id: 0,
                    },
                )
                .unwrap();
            assert_eq!(res, 16);

            // move forward in time
            app.set_block(BlockInfo {
                height: current_block.clone() + 100,
                time: current_time.clone().plus_seconds(130),
                chain_id: chainid.clone(),
            });

            // try to claim ticket before randomness is requested
            let claim_but_no_randomness_yet = app
                .execute_contract(
                    Addr::unchecked("wallet-1".to_string()),
                    raffle_contract_addr.clone(),
                    &ExecuteMsg::ClaimNft { raffle_id: 0 },
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
                    Addr::unchecked("wallet-1"),
                    raffle_contract_addr.clone(),
                    &ExecuteMsg::NoisReceive {
                        callback: NoisCallback {
                            job_id: "raffle".to_string(),
                            published: current_time.clone(),
                            randomness: HexBinary::from_hex(
                                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa115",
                            )
                            .unwrap(),
                        },
                        raffle_id: 0,
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
                    raffle_contract_addr.clone(),
                    &ExecuteMsg::NoisReceive {
                        callback: NoisCallback {
                            job_id: "raffle".to_string(),
                            published: current_time.clone(),
                            randomness: HexBinary::from_hex(
                                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa115",
                            )
                            .unwrap(),
                        },
                        raffle_id: 0,
                    },
                    &[],
                )
                .unwrap();

            // claims the ticket
            let _claim_ticket = app
                .execute_contract(
                    Addr::unchecked("wallet-1".to_string()),
                    raffle_contract_addr.clone(),
                    &ExecuteMsg::ClaimNft { raffle_id: 0 },
                    &[],
                )
                .unwrap();
            // println!("{:#?}", _claim_ticket);
            let res: RaffleResponse = app
                .wrap()
                .query_wasm_smart(
                    raffle_contract_addr.clone(),
                    &raffles::msg::QueryMsg::RaffleInfo { raffle_id: 0 },
                )
                .unwrap();
            assert_eq!(res.raffle_state, RaffleState::Claimed);

        }
    }
}
