#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coin, Addr, BlockInfo, Coin, Decimal, Empty, HexBinary, Timestamp, Uint128,
    };
    use cw_multi_test::Executor;
    use cw_multi_test::{BankSudo, SudoMsg};
    use nois::NoisCallback;
    use raffles::{
        error::ContractError,
        msg::{ConfigResponse, ExecuteMsg},
        state::{RaffleInfo, RaffleOptions, RaffleOptionsMsg, RaffleState},
    };

    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};
    use vending_factory::msg::VendingMinterCreateMsg;

    use raffles::{msg::InstantiateMsg, state::NOIS_AMOUNT};
    #[cfg(feature = "sg")]
    use {sg721::CollectionInfo, sg_multi_test::StargazeApp};

    use crate::common_setup::{
        contract_boxes::{
            contract_raffles, contract_sg721_base, contract_vending_factory,
            contract_vending_minter, custom_mock_app,
        },
        helpers::assert_error,
        setup_minter::common::constants::{
            CREATION_FEE_AMNT, NOIS_PROXY_ADDR, OWNER_ADDR, RAFFLE_NAME, SG721_CONTRACT,
            VENDING_MINTER,
        },
    };
    use vending_factory::state::{ParamsExtension, VendingMinterParams};

    pub fn proper_instantiate_raffles_with_limits() -> (StargazeApp, Addr, Addr) {
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
                    name: RAFFLE_NAME.to_string(),
                    nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
                    nois_proxy_coin: coin(NOIS_AMOUNT.into(), NATIVE_DENOM.to_string()),
                    owner: Some(OWNER_ADDR.to_string()),
                    fee_addr: Some(OWNER_ADDR.to_owned()),
                    minimum_raffle_duration: Some(20),
                    minimum_raffle_timeout: Some(420),
                    max_participant_number: Some(3),
                    raffle_fee: Decimal::percent(50),
                    creation_coins: vec![
                        coin(CREATION_FEE_AMNT.into(), NATIVE_DENOM.to_string()),
                        coin(CREATION_FEE_AMNT.into(), "usstars".to_string()),
                    ]
                    .into(),
                },
                &[],
                "raffle",
                Some(Addr::unchecked(OWNER_ADDR).to_string()),
            )
            .unwrap();

        (app, raffle_contract_addr, factory_addr)
    }

    #[test]
    fn can_init() {
        // create testing app
        let (mut app, raffle_contract_addr, factory_addr) =
            proper_instantiate_raffles_with_limits();

        let query_config: raffles::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_contract_addr.clone(),
                &raffles::msg::QueryMsg::Config {},
            )
            .unwrap();

        // println!("{:#?}", query_config);
        assert_eq!(
            query_config,
            ConfigResponse {
                name: RAFFLE_NAME.to_string(),
                owner: Addr::unchecked(OWNER_ADDR),
                fee_addr: Addr::unchecked(OWNER_ADDR),
                last_raffle_id: 0,
                minimum_raffle_duration: 20,
                minimum_raffle_timeout: 420,
                raffle_fee: Decimal::percent(50),
                lock: false,
                nois_proxy_addr: Addr::unchecked("nois"),
                nois_proxy_coin: coin(500000, NATIVE_DENOM),
                creation_coins: vec![
                    coin(CREATION_FEE_AMNT, NATIVE_DENOM.to_string()),
                    coin(CREATION_FEE_AMNT, "usstars".to_string())
                ]
            }
        );

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

        let _raffle_code_id = app.store_code(contract_raffles());

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
                            external_link: Some("https://example.com/external.html".to_string()),
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
        let _mint_nft_tokens2 = app
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
        let _mint_nft_tokens3 = app
            .execute_contract(
                Addr::unchecked("wallet-1"),
                Addr::unchecked(VENDING_MINTER),
                &vending_minter::msg::ExecuteMsg::Mint {},
                &[Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100000u128),
                }],
            )
            .unwrap();

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
        // token id 56
        let _grant_approval = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                Addr::unchecked(SG721_CONTRACT),
                &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                    spender: raffle_contract_addr.to_string(),
                    token_id: "56".to_string(),
                    expires: None,
                },
                &[],
            )
            .unwrap();
        // token id 49
        let _grant_approval = app
            .execute_contract(
                Addr::unchecked("wallet-1"),
                Addr::unchecked(SG721_CONTRACT),
                &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                    spender: raffle_contract_addr.to_string(),
                    token_id: "49".to_string(),
                    expires: None,
                },
                &[],
            )
            .unwrap();

        let _good_create_raffle = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                raffle_contract_addr.clone(),
                &ExecuteMsg::CreateRaffle {
                    owner: None,
                    assets: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "41".to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "56".to_string(),
                        }),
                    ],
                    raffle_options: RaffleOptionsMsg {
                        raffle_start_timestamp: Some(Timestamp::from_nanos(1647032600000000000)),
                        raffle_duration: None,
                        raffle_timeout: None,
                        comment: None,
                        max_participant_number: Some(3),
                        max_ticket_per_address: Some(1),
                        raffle_preview: None,
                    },
                    raffle_ticket_price: AssetInfo::Coin(Coin {
                        denom: "ustars".to_string(),
                        amount: Uint128::new(100u128),
                    }),
                    autocycle: Some(false),
                },
                &[coin(50, "ustars")],
            )
            .unwrap();

        let res: raffles::msg::RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_contract_addr.clone(),
                &raffles::msg::QueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();

        // verify contract response
        assert_eq!(
            res.clone().raffle_info.unwrap(),
            RaffleInfo {
                owner: Addr::unchecked(OWNER_ADDR),
                assets: vec![
                    AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "41".to_string(),
                    }),
                    AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "56".to_string(),
                    })
                ],
                raffle_ticket_price: AssetInfo::Coin(Coin::new(100, "ustars".to_string())),
                number_of_tickets: 0,
                randomness: None,
                winner: None,
                is_cancelled: false,
                raffle_options: RaffleOptions {
                    raffle_start_timestamp: Timestamp::from_nanos(1647032600000000000),
                    raffle_duration: 20,
                    raffle_timeout: 420,
                    comment: None,
                    max_participant_number: Some(3),
                    max_ticket_per_address: Some(1),
                    raffle_preview: 0,
                }
            }
        );

        // move forward in time
        app.set_block(BlockInfo {
            height: current_block.clone() + 1,
            time: current_time.clone().plus_nanos(200000000000),
            chain_id: chainid.clone(),
        });

        // ensure error if max tickets per address set is reached
        let bad_ticket_purchase = app
            .execute_contract(
                Addr::unchecked("wallet-1"),
                raffle_contract_addr.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 2,
                    sent_assets: AssetInfo::Coin(Coin::new(200, "ustars".to_string())),
                },
                &[Coin::new(200, "ustars".to_string())],
            )
            .unwrap_err();
        assert_error(
            Err(bad_ticket_purchase),
            ContractError::TooMuchTicketsForUser {
                max: 1,
                nb_before: 0,
                nb_after: 2,
            }
            .to_string(),
        );

        // ensure raffle duration
        // move forward in time
        app.set_block(BlockInfo {
            height: current_block.clone() + 100,
            time: current_time.clone().plus_seconds(1000),
            chain_id: chainid.clone(),
        });

        // ensure raffle ends correctly
        let bad_ticket_purchase = app
            .execute_contract(
                Addr::unchecked("wallet-1"),
                raffle_contract_addr.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 1,
                    sent_assets: AssetInfo::Coin(Coin::new(100, "ustars".to_string())),
                },
                &[Coin::new(100, "ustars".to_string())],
            )
            .unwrap_err();
        assert_error(
            Err(bad_ticket_purchase),
            ContractError::CantBuyTickets {}.to_string(),
        );

        // simulates the response from nois_proxy
        let _good_receive_randomness = app
            .execute_contract(
                Addr::unchecked(NOIS_PROXY_ADDR),
                raffle_contract_addr.clone(),
                &ExecuteMsg::NoisReceive {
                    callback: NoisCallback {
                        job_id: "raffle-0".to_string(),
                        published: current_time.clone(),
                        randomness: HexBinary::from_hex(
                            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa420",
                        )
                        .unwrap(),
                    },
                },
                &[],
            )
            .unwrap();

        // assert that if no raffles are bought, raffle is finished and there is no winner
        let res: raffles::msg::RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                raffle_contract_addr.clone(),
                &raffles::msg::QueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();
        // println!("{:#?}", res);
        assert_eq!(res.clone().raffle_state, RaffleState::Finished);
        assert_eq!(res.raffle_info.unwrap().winner, None);

        // assert the tokens being raffled are sent back to owner if no tickets are purchased,
        // even if someone else calls the contract to determine winner tokens
        let _claim_ticket = app
            .execute_contract(
                Addr::unchecked("wallet-1".to_string()),
                raffle_contract_addr.clone(),
                &ExecuteMsg::DetermineWinner { raffle_id: 0 },
                &[],
            )
            .unwrap();

        let res: cw721::OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                SG721_CONTRACT.to_string(),
                &sg721_base::QueryMsg::OwnerOf {
                    token_id: "41".to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        println!("{:#?}", res);
        assert_eq!(res.owner, Addr::unchecked(OWNER_ADDR));
    }
}
