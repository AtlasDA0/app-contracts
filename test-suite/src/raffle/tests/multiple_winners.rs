#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Uint128};
    use cw_multi_test::Executor;
    use raffles::{msg::ExecuteMsg, state::RaffleOptionsMsg};

    use utils::state::{AssetInfo, Sg721Token};

    use crate::common_setup::setup_accounts_and_block::setup_accounts;
    use crate::common_setup::setup_accounts_and_block::setup_n_accounts;
    use crate::common_setup::setup_raffle::proper_raffle_instantiate;

    use crate::common_setup::setup_minter::common::constants::OWNER_ADDR;
    use crate::common_setup::setup_raffle::proper_raffle_instantiate_precise;
    use crate::raffle::setup::helpers::finish_raffle_timeout_generic;
    use crate::raffle::setup::helpers::mint_additional_token;
    use crate::raffle::setup::helpers::mint_one_token;
    use crate::raffle::setup::helpers::{finish_raffle_timeout, raffle_info};

    #[test]
    fn multiple_winners() {
        // create testing app
        let (mut app, contracts) = proper_raffle_instantiate();
        let (_owner_addr, one, two) = setup_accounts(&mut app);

        let current_time = app.block_info().time;

        let token = mint_one_token(&mut app, &contracts);
        let token1 = mint_additional_token(&mut app, &contracts, &token);

        let _good_create_raffle = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                contracts.raffle.clone(),
                &ExecuteMsg::CreateRaffle {
                    owner: None,
                    assets: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token.nft.to_string(),
                            token_id: token.token_id.to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token1.nft.to_string(),
                            token_id: token1.token_id.to_string(),
                        }),
                    ],
                    raffle_options: RaffleOptionsMsg {
                        raffle_start_timestamp: Some(current_time),
                        raffle_duration: None,
                        comment: None,
                        max_ticket_number: Some(3),
                        max_ticket_per_address: Some(1),
                        raffle_preview: None,
                        one_winner_per_asset: true,
                        min_ticket_number: None,
                        whitelist: None,
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

        let res = raffle_info(&app, &contracts, 0);

        // verify contract response
        assert!(
            res.clone()
                .raffle_info
                .unwrap()
                .raffle_options
                .one_winner_per_asset
        );

        // ensure raffle ends correctly
        let _ticket_purchase_1 = app
            .execute_contract(
                one.clone(),
                contracts.raffle.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 1,
                    sent_assets: AssetInfo::Coin(Coin::new(100, "ustars".to_string())),
                    on_behalf_of: None,
                },
                &[Coin::new(100, "ustars".to_string())],
            )
            .unwrap();
        let _ticket_purchase_2 = app
            .execute_contract(
                two.clone(),
                contracts.raffle.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 1,
                    sent_assets: AssetInfo::Coin(Coin::new(100, "ustars".to_string())),
                    on_behalf_of: None,
                },
                &[Coin::new(100, "ustars".to_string())],
            )
            .unwrap();
        finish_raffle_timeout(&mut app, &contracts, 0, 1000).unwrap();

        let res: cw721::OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token.nft.to_string(),
                &sg721_base::QueryMsg::OwnerOf {
                    token_id: token.token_id.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, two.to_string());
        let res: cw721::OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token1.nft.to_string(),
                &sg721_base::QueryMsg::OwnerOf {
                    token_id: token1.token_id.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, one.to_string());
    }

    #[test]
    fn not_enough_participants() {
        // create testing app
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, one, _) = setup_accounts(&mut app);

        let current_time = app.block_info().time;

        let token = mint_one_token(&mut app, &contracts);
        let token1 = mint_additional_token(&mut app, &contracts, &token);
        let token2 = mint_additional_token(&mut app, &contracts, &token);

        let _good_create_raffle = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                contracts.raffle.clone(),
                &ExecuteMsg::CreateRaffle {
                    owner: None,
                    assets: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token.nft.to_string(),
                            token_id: token.token_id.to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token1.nft.to_string(),
                            token_id: token1.token_id.to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token2.nft.to_string(),
                            token_id: token2.token_id.to_string(),
                        }),
                    ],
                    raffle_options: RaffleOptionsMsg {
                        raffle_start_timestamp: Some(current_time),
                        raffle_duration: None,
                        comment: None,
                        max_ticket_number: Some(3),
                        max_ticket_per_address: Some(1),
                        raffle_preview: None,
                        one_winner_per_asset: true,
                        min_ticket_number: None,
                        whitelist: None,
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

        let res = raffle_info(&app, &contracts, 0);

        // verify contract response
        assert!(
            res.clone()
                .raffle_info
                .unwrap()
                .raffle_options
                .one_winner_per_asset
        );
        assert_eq!(
            res.clone()
                .raffle_info
                .unwrap()
                .raffle_options
                .min_ticket_number,
            Some(3)
        );

        // ensure raffle ends correctly
        let _ticket_purchase_1 = app
            .execute_contract(
                one.clone(),
                contracts.raffle.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 1,
                    sent_assets: AssetInfo::Coin(Coin::new(100, "ustars".to_string())),
                    on_behalf_of: None,
                },
                &[Coin::new(100, "ustars".to_string())],
            )
            .unwrap();

        assert_eq!(
            raffle_info(&app, &contracts, 0)
                .clone()
                .raffle_info
                .unwrap()
                .number_of_tickets,
            1
        );

        finish_raffle_timeout(&mut app, &contracts, 0, 1000).unwrap();

        let res: cw721::OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token.nft.to_string(),
                &sg721_base::QueryMsg::OwnerOf {
                    token_id: token.token_id.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();

        assert_eq!(res.owner, owner_addr.to_string());
        let res: cw721::OwnerOfResponse = app
            .wrap()
            .query_wasm_smart(
                token1.nft.to_string(),
                &sg721_base::QueryMsg::OwnerOf {
                    token_id: token1.token_id.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(res.owner, owner_addr.to_string());
    }

    fn test_n_randomness(n: u64, randomness_id: u8) {
        // create testing app
        let (mut app, contracts) = proper_raffle_instantiate_precise(None);
        let (owner, _, _) = setup_accounts(&mut app);
        let participants = setup_n_accounts(&mut app, n);

        let current_time = app.block_info().time;

        let token = mint_one_token(&mut app, &contracts);
        let token1 = mint_additional_token(&mut app, &contracts, &token);
        let token2 = mint_additional_token(&mut app, &contracts, &token);

        let _good_create_raffle = app
            .execute_contract(
                owner,
                contracts.raffle.clone(),
                &ExecuteMsg::CreateRaffle {
                    owner: None,
                    assets: vec![
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token.nft.to_string(),
                            token_id: token.token_id.to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token1.nft.to_string(),
                            token_id: token1.token_id.to_string(),
                        }),
                        AssetInfo::Sg721Token(Sg721Token {
                            address: token2.nft.to_string(),
                            token_id: token2.token_id.to_string(),
                        }),
                    ],
                    raffle_options: RaffleOptionsMsg {
                        raffle_start_timestamp: Some(current_time),
                        raffle_duration: None,
                        comment: None,
                        max_ticket_number: None,
                        max_ticket_per_address: None,
                        raffle_preview: None,
                        one_winner_per_asset: true,
                        min_ticket_number: None,
                        whitelist: None,
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

        // every participant buys 20 tickets
        for addr in participants {
            app.execute_contract(
                addr.clone(),
                contracts.raffle.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 20,
                    sent_assets: AssetInfo::Coin(Coin::new(20 * 100, "ustars".to_string())),
                    on_behalf_of: None,
                },
                &[Coin::new(20 * 100, "ustars".to_string())],
            )
            .unwrap();
        }

        finish_raffle_timeout_generic(&mut app, &contracts, 0, 1000, randomness_id).unwrap();
    }

    #[test]
    fn random_with_default_randomness() {
        test_n_randomness(5, 0);
    }

    #[test]
    fn random_with_other() {
        test_n_randomness(5, 1);
    }
}
