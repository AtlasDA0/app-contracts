#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, BlockInfo, Coin, Decimal, Timestamp, Uint128};
    use cw_multi_test::Executor;
    use cw_multi_test::{BankSudo, SudoMsg};

    use raffles::state::MINIMUM_RAFFLE_DURATION;
    use raffles::{
        error::ContractError,
        msg::{ConfigResponse, ExecuteMsg},
        state::{RaffleInfo, RaffleOptions, RaffleOptionsMsg, RaffleState},
    };

    use utils::state::{AssetInfo, Locks, Sg721Token, NATIVE_DENOM};

    use crate::common_setup::nois_proxy::{NOIS_AMOUNT, NOIS_DENOM};
    use crate::common_setup::setup_minter::common::constants::{
        CREATION_FEE_AMNT_NATIVE, CREATION_FEE_AMNT_STARS, TREASURY_ADDR,
    };
    use crate::common_setup::setup_raffle::proper_raffle_instantiate_precise;
    use crate::common_setup::{
        contract_boxes::contract_raffles,
        helpers::assert_error,
        setup_minter::common::constants::{OWNER_ADDR, RAFFLE_NAME},
    };
    use crate::raffle::setup::helpers::{
        finish_raffle_timeout, mint_additional_token, mint_one_token,
    };

    #[test]
    fn finished_when_all_tickets_sold() {
        // create testing app
        let (mut app, contracts) = proper_raffle_instantiate_precise(Some(80), None);
        let token = mint_one_token(&mut app, &contracts);

        let _current_time = app.block_info().time;
        let current_block = app.block_info().height;
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

        let _raffle_code_id = app.store_code(contract_raffles());

        let block_info = app.block_info();
        let _good_create_raffle = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                contracts.raffle.clone(),
                &ExecuteMsg::CreateRaffle {
                    owner: None,
                    assets: vec![AssetInfo::Sg721Token(Sg721Token {
                        address: token.nft.to_string(),
                        token_id: token.token_id.clone(),
                    })],
                    raffle_options: RaffleOptionsMsg {
                        raffle_start_timestamp: Some(block_info.time),
                        raffle_duration: None,

                        comment: None,
                        max_ticket_number: Some(80),
                        max_ticket_per_address: Some(80),
                        raffle_preview: None,
                        min_ticket_number: None,
                        one_winner_per_asset: false,
                        gating_raffle: vec![],
                    },
                    raffle_ticket_price: AssetInfo::Coin(Coin {
                        denom: "ustars".to_string(),
                        amount: Uint128::new(100u128),
                    }),
                },
                &[coin(50, "ustars")],
            )
            .unwrap();

        // We can still buy tickets long in the future

        // ensure raffle ends correctly
        let _good_ticket_purchase = app
            .execute_contract(
                Addr::unchecked("wallet-1"),
                contracts.raffle.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 80,
                    sent_assets: AssetInfo::Coin(Coin::new(8000, "ustars".to_string())),
                    on_behalf_of: None,
                },
                &[Coin::new(8000, "ustars".to_string())],
            )
            .unwrap();

        // move forward in time for the timeout period
        let current_time = app.block_info().time;
        app.set_block(BlockInfo {
            height: current_block + 1,
            time: current_time.plus_seconds(420),
            chain_id: chainid.clone(),
        });

        // assert that if no raffles are bought, raffle is finished and there is no winner
        let res: raffles::msg::RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                contracts.raffle.clone(),
                &raffles::msg::QueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();

        assert_eq!(res.clone().raffle_state, RaffleState::Closed);
        assert_eq!(res.raffle_info.unwrap().winners.len(), 0);

        // assert the tokens being raffled are sent back to owner if no tickets are purchased,
        // even if someone else calls the contract to determine winner tokens
        finish_raffle_timeout(&mut app, &contracts, 0, MINIMUM_RAFFLE_DURATION + 1).unwrap();

        let res: cw721::OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token.nft.to_string(),
                &sg721_base::QueryMsg::OwnerOf {
                    token_id: token.token_id,
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, Addr::unchecked("wallet-1"));
    }
    #[test]
    fn can_init() {
        // create testing app
        let (mut app, contracts) = proper_raffle_instantiate_precise(Some(80), None);

        let query_config: raffles::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(contracts.raffle.clone(), &raffles::msg::QueryMsg::Config {})
            .unwrap();

        assert_eq!(
            query_config,
            ConfigResponse {
                name: RAFFLE_NAME.to_string(),
                owner: Addr::unchecked(OWNER_ADDR),
                fee_addr: Addr::unchecked(TREASURY_ADDR),
                last_raffle_id: 0,
                minimum_raffle_duration: 1,
                raffle_fee: Decimal::percent(50),
                locks: Locks {
                    lock: false,
                    sudo_lock: false,
                },
                nois_proxy_addr: contracts.nois.clone(),
                nois_proxy_coin: coin(NOIS_AMOUNT, NOIS_DENOM),
                creation_coins: vec![
                    coin(CREATION_FEE_AMNT_NATIVE, NATIVE_DENOM.to_string()),
                    coin(CREATION_FEE_AMNT_STARS, "ustars".to_string())
                ],
            }
        );

        let current_time = app.block_info().time;
        let current_block = app.block_info().height;
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

        let token1 = mint_one_token(&mut app, &contracts);
        let token2 = mint_additional_token(&mut app, &contracts, &token1);
        let _token3 = mint_additional_token(&mut app, &contracts, &token1);
        let _good_create_raffle = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                contracts.raffle.clone(),
                &ExecuteMsg::CreateRaffle {
                    owner: None,
                    assets: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token1.nft.to_string(),
                            token_id: token1.token_id.clone(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token2.nft.to_string(),
                            token_id: token2.token_id.clone(),
                        }),
                    ],
                    raffle_options: RaffleOptionsMsg {
                        raffle_start_timestamp: Some(Timestamp::from_nanos(1647032600000000000)),
                        raffle_duration: None,

                        comment: None,
                        max_ticket_number: Some(3),
                        max_ticket_per_address: Some(1),
                        raffle_preview: None,
                        one_winner_per_asset: false,
                        gating_raffle: vec![],
                        min_ticket_number: None,
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
                contracts.raffle.clone(),
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
                        address: token1.nft.to_string(),
                        token_id: token1.token_id.clone(),
                    }),
                    AssetInfo::Sg721Token(Sg721Token {
                        address: token2.nft.to_string(),
                        token_id: token2.token_id.clone(),
                    })
                ],
                raffle_ticket_price: AssetInfo::Coin(Coin::new(100, "ustars".to_string())),
                number_of_tickets: 0,
                randomness: None,
                winners: vec![],
                is_cancelled: false,
                raffle_options: RaffleOptions {
                    raffle_start_timestamp: Timestamp::from_nanos(1647032600000000000),
                    raffle_duration: 1,
                    comment: None,
                    max_ticket_number: Some(3),
                    max_ticket_per_address: Some(1),
                    raffle_preview: 0,
                    one_winner_per_asset: false,
                    gating_raffle: vec![],
                    min_ticket_number: None,
                }
            }
        );

        // move forward in time
        app.set_block(BlockInfo {
            height: current_block + 1,
            time: current_time.clone().plus_nanos(200000000000),
            chain_id: chainid.clone(),
        });

        // ensure raffle duration
        // move forward in time
        app.set_block(BlockInfo {
            height: current_block + 100,
            time: current_time.clone().plus_seconds(1000),
            chain_id: chainid.clone(),
        });

        // ensure raffle ends correctly
        let bad_ticket_purchase = app
            .execute_contract(
                Addr::unchecked("wallet-1"),
                contracts.raffle.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 1,
                    sent_assets: AssetInfo::Coin(Coin::new(100, "ustars".to_string())),
                    on_behalf_of: None,
                },
                &[Coin::new(100, "ustars".to_string())],
            )
            .unwrap_err();
        assert_error(
            Err(bad_ticket_purchase),
            ContractError::CantBuyTickets {}.to_string(),
        );

        // assert that if no raffles are bought, raffle is finished and there is no winner
        let res: raffles::msg::RaffleResponse = app
            .wrap()
            .query_wasm_smart(
                contracts.raffle.clone(),
                &raffles::msg::QueryMsg::RaffleInfo { raffle_id: 0 },
            )
            .unwrap();
        assert_eq!(res.clone().raffle_state, RaffleState::Closed);
        assert_eq!(res.raffle_info.unwrap().winners.len(), 0);

        // assert the tokens being raffled are sent back to owner if no tickets are purchased,
        // even if someone else calls the contract to determine winner tokens
        finish_raffle_timeout(&mut app, &contracts, 0, 10 + 1).unwrap();

        let res: cw721::OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token1.nft.to_string(),
                &sg721_base::QueryMsg::OwnerOf {
                    token_id: token1.token_id,
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, Addr::unchecked(OWNER_ADDR));
    }
}
