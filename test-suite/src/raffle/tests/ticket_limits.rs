#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, BlockInfo, Coin, Decimal, Timestamp, Uint128};
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
                    minimum_raffle_duration: Some(20),
                    minimum_raffle_timeout: Some(420),
                    max_participant_number: Some(3),
                    raffle_fee: Some(Decimal::percent(50)),
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
            msg::ExecuteMsg,
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

            // println!("{:#?}", query_config);
            assert_eq!(query_config.clone().owner, Addr::unchecked("fee"));
            assert_eq!(query_config.clone().fee_addr, Addr::unchecked("fee"));
            assert_eq!(query_config.clone().minimum_raffle_duration, 20);
            assert_eq!(query_config.clone().minimum_raffle_timeout, 420);
            assert_eq!(query_config.clone().minimum_raffle_timeout, 420);
            assert_eq!(query_config.clone().creation_fee_amount, Uint128::new(50));
            assert_eq!(
                query_config.clone().creation_fee_denom,
                vec!["ustars".to_string(), "usstars".to_string()]
            );
            assert_eq!(query_config.clone().raffle_fee, Decimal::percent(50));
            assert_eq!(query_config.clone().lock, false);
            assert_eq!(
                query_config.clone().nois_proxy_addr,
                Addr::unchecked("nois")
            );
            assert_eq!(query_config.clone().nois_proxy_denom, "ustars".to_string());
            assert_eq!(query_config.clone().nois_proxy_amount, Uint128::new(50));

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

            // create a raffle, ensure:
            // - timeout
            // - max participants
            // - start time

            let _good_create_raffle = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    raffle_contract_addr.clone(),
                    &raffles::msg::ExecuteMsg::CreateRaffle {
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
                            raffle_start_timestamp: Some(Timestamp::from_nanos(
                                1647032600000000000,
                            )),
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
            // verify owner defaults to info.sender if None
            assert_eq!(
                res.clone().raffle_info.unwrap().owner,
                Addr::unchecked(OWNER_ADDR)
            );
            // verify array of tokens being raffled
            assert_eq!(
                res.clone().raffle_info.unwrap().assets,
                vec![
                    AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "41".to_string(),
                    }),
                    AssetInfo::Sg721Token(Sg721Token {
                        address: SG721_CONTRACT.to_string(),
                        token_id: "56".to_string(),
                    })
                ]
            );
            // verify raffle ticket price
            assert_eq!(
                res.clone().raffle_info.unwrap().raffle_ticket_price,
                AssetInfo::Coin(Coin::new(100, "ustars".to_string()))
            );
            // verify initial # of tickets
            assert_eq!(res.clone().raffle_info.unwrap().number_of_tickets, 0);
            // verify no randomness
            assert_eq!(res.clone().raffle_info.unwrap().randomness, None);
            // verify no winner
            assert_eq!(res.clone().raffle_info.unwrap().winner, None);
            // verify not cancelled
            assert_eq!(res.clone().raffle_info.unwrap().is_cancelled, false);
            // verify raffle delayed start
            assert_eq!(
                res.clone()
                    .raffle_info
                    .unwrap()
                    .raffle_options
                    .raffle_start_timestamp,
                Timestamp::from_nanos(1647032600000000000)
            );
            // verify max default max participant
            assert_eq!(
                res.clone()
                    .raffle_info
                    .unwrap()
                    .raffle_options
                    .max_participant_number,
                Some(3)
            );
            // verify max_ticket_per_address
            assert_eq!(
                res.clone()
                    .raffle_info
                    .unwrap()
                    .raffle_options
                    .max_ticket_per_address,
                Some(1)
            );
            // verify duration
            assert_eq!(
                res.clone()
                    .raffle_info
                    .unwrap()
                    .raffle_options
                    .raffle_duration,
                20
            );
            //verify raffle timeout
            assert_eq!(
                res.clone()
                    .raffle_info
                    .unwrap()
                    .raffle_options
                    .raffle_timeout,
                420
            );

            // move forward in time
            app.set_block(BlockInfo {
                height: current_block.clone() + 1,
                time: current_time.clone().plus_nanos(200000000000),
                chain_id: chainid.clone(),
            });

            // ensure max tickets per address
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
            // even if someone else calls the contract to claim tokens 
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
}
