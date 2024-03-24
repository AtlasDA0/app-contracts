#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Uint128};
    use cw_multi_test::Executor;
    use cw_multi_test::{BankSudo, SudoMsg};
    use raffles::{msg::ExecuteMsg, state::RaffleOptionsMsg};

    use utils::state::{AssetInfo, Sg721Token};

    use crate::common_setup::setup_raffle::proper_raffle_instantiate;

    use crate::common_setup::setup_minter::common::constants::OWNER_ADDR;
    use crate::raffle::setup::helpers::finish_raffle_timeout;
    use crate::raffle::setup::helpers::mint_additional_token;
    use crate::raffle::setup::helpers::mint_one_token;

    #[test]
    fn multiple_winners() {
        // create testing app
        let (mut app, contracts) = proper_raffle_instantiate();

        let current_time = app.block_info().time;
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
                Addr::unchecked("wallet-1"),
                contracts.raffle.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 1,
                    sent_assets: AssetInfo::Coin(Coin::new(100, "ustars".to_string())),
                },
                &[Coin::new(100, "ustars".to_string())],
            )
            .unwrap();
        let _ticket_purchase_2 = app
            .execute_contract(
                Addr::unchecked("wallet-2"),
                contracts.raffle.clone(),
                &ExecuteMsg::BuyTicket {
                    raffle_id: 0,
                    ticket_count: 1,
                    sent_assets: AssetInfo::Coin(Coin::new(100, "ustars".to_string())),
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
        assert_eq!(res.owner, "wallet-1".to_string());
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
        assert_eq!(res.owner, "wallet-2".to_string());
    }
}
