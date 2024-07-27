#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Decimal, Empty, Uint128};
    use cw_multi_test::Executor;
    use raffles::{
        execute::NOIS_TIMEOUT,
        state::{FeeDiscountMsg, MINIMUM_RAFFLE_DURATION},
    };
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    use crate::{
        common_setup::{
            app::StargazeApp,
            msg::RaffleContracts,
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_minter::common::constants::OWNER_ADDR,
            setup_raffle::proper_raffle_instantiate_precise,
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_setup},
            helpers::{finish_raffle_timeout, mint_one_token, raffle_info},
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };

    pub fn proper_raffle_instantiate(
        max_ticket_number: Option<u32>,
        nft_owner: String,
    ) -> (StargazeApp, RaffleContracts) {
        let (mut app, contracts) = proper_raffle_instantiate_precise(max_ticket_number, None);
        let nft = mint_one_token(&mut app, &contracts);

        app.execute_contract(
            Addr::unchecked(OWNER_ADDR),
            nft.nft.clone(),
            &sg721_base::msg::ExecuteMsg::<Empty, Empty>::TransferNft {
                recipient: nft_owner,
                token_id: nft.token_id.to_string(),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            Addr::unchecked(OWNER_ADDR),
            contracts.raffle.clone(),
            &raffles::msg::ExecuteMsg::UpdateConfig {
                name: None,
                owner: None,
                fee_addr: None,
                minimum_raffle_duration: None,
                max_tickets_per_raffle: None,
                raffle_fee: None,
                nois_proxy_addr: None,
                nois_proxy_coin: None,
                creation_coins: None,
                fee_discounts: Some(vec![
                    FeeDiscountMsg {
                        discount: Decimal::one(),
                        condition: raffles::state::AdvantageOptionsMsg::Sg721Token {
                            nft_count: 1,
                            nft_address: nft.nft.to_string(),
                        },
                    },
                    FeeDiscountMsg {
                        discount: Decimal::percent(50),
                        condition: raffles::state::AdvantageOptionsMsg::Staking {
                            min_voting_power: Uint128::from(100u128),
                        },
                    },
                ]),
                locality_config: None,
            },
            &[],
        )
        .unwrap();

        (app, contracts)
    }

    #[test]
    pub fn owner_has_nft() {
        let (mut app, contracts) = proper_raffle_instantiate(None, OWNER_ADDR.to_string());
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let nft = mint_one_token(&mut app, &contracts);
        // create raffle

        let approvals: cw721::ApprovalsResponse = app
            .wrap()
            .query_wasm_smart(
                nft.nft.clone(),
                &sg721_base::msg::QueryMsg::Approvals {
                    token_id: nft.token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        print!("{:?}", approvals);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: nft.nft.to_string(),
                token_id: nft.token_id.clone(),
            })],
            duration: None,
            max_tickets: None,
            min_ticket_number: None,
            gating: vec![],
        };

        create_raffle_setup(params).unwrap();

        // Purchasing tickets for 3 people
        // ensure error if max tickets per address set is reached
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![three.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let owner_balance_before = app.wrap().query_balance(&owner_addr, "ustars").unwrap();

        finish_raffle_timeout(
            &mut app,
            &contracts,
            0,
            MINIMUM_RAFFLE_DURATION + NOIS_TIMEOUT,
        )
        .unwrap();

        // queries the raffle
        let res = raffle_info(&app, &contracts, 0);

        // verify winner is always owner
        assert_eq!(
            two,
            res.raffle_info.unwrap().winners[0],
            "winner should be one of the contestants"
        );
        // verify no tickets can be bought after raffle ends
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let purchase_tickets = buy_tickets_template(params);
        assert!(
            purchase_tickets.is_err(),
            "There should be an issue with purchasing a ticket once the winner is determined"
        );

        // We make sure the owner has more balance
        let owner_balance_after = app.wrap().query_balance(&owner_addr, "ustars").unwrap();

        assert_eq!(
            owner_balance_before.amount
                + Decimal::percent(100) * (Uint128::from(4u128) * Uint128::from(3u128)), // 0% fee for NFT owners
            owner_balance_after.amount
        );
    }

    #[test]
    pub fn owner_is_sufficient_staker() {
        let (mut app, contracts) = proper_raffle_instantiate(None, "any".to_string());
        let (owner_addr, _one, _) = setup_accounts(&mut app);
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let nft = mint_one_token(&mut app, &contracts);

        app.execute(
            owner_addr.clone(),
            cosmwasm_std::CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate {
                validator: "validator".to_string(),
                amount: coin(150, "TOKEN"),
            }),
        )
        .unwrap();
        // create raffle

        let approvals: cw721::ApprovalsResponse = app
            .wrap()
            .query_wasm_smart(
                nft.nft.clone(),
                &sg721_base::msg::QueryMsg::Approvals {
                    token_id: nft.token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        print!("{:?}", approvals);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: nft.nft.to_string(),
                token_id: nft.token_id.clone(),
            })],
            duration: None,
            max_tickets: None,
            min_ticket_number: None,
            gating: vec![],
        };

        create_raffle_setup(params).unwrap();

        // Purchasing tickets for 3 people
        // ensure error if max tickets per address set is reached
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![three.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let owner_balance_before = app.wrap().query_balance(&owner_addr, "ustars").unwrap();

        finish_raffle_timeout(
            &mut app,
            &contracts,
            0,
            MINIMUM_RAFFLE_DURATION + NOIS_TIMEOUT,
        )
        .unwrap();

        // queries the raffle
        let res = raffle_info(&app, &contracts, 0);

        // verify winner is always owner
        assert_eq!(
            two,
            res.raffle_info.unwrap().winners[0],
            "winner should always be the owner if no tickets were bought"
        );
        // verify no tickets can be bought after raffle ends
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let purchase_tickets = buy_tickets_template(params);
        assert!(
            purchase_tickets.is_err(),
            "There should be an issue with purchasing a ticket once the winner is determined"
        );

        // We make sure the owner has more balance
        let owner_balance_after = app.wrap().query_balance(&owner_addr, "ustars").unwrap();

        assert_eq!(
            owner_balance_before.amount
                + (Decimal::percent(100) - Decimal::percent(50) * Decimal::percent(50))
                    * (Uint128::from(4u128) * Uint128::from(3u128)), // 50% fee * 50% fee for delegators
            owner_balance_after.amount
        );
    }

    #[test]
    pub fn owner_is_not_sufficient_staker() {
        let (mut app, contracts) = proper_raffle_instantiate(None, "any".to_string());

        let (owner_addr, _one, _) = setup_accounts(&mut app);
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let nft = mint_one_token(&mut app, &contracts);

        // app.execute(
        //     owner_addr.clone(),
        //     cosmwasm_std::CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate {
        //         validator: "validator".to_string(),
        //         amount: coin(50, "TOKEN"),
        //     }),
        // )
        // .unwrap();
        // create raffle

        let approvals: cw721::ApprovalsResponse = app
            .wrap()
            .query_wasm_smart(
                nft.nft.clone(),
                &sg721_base::msg::QueryMsg::Approvals {
                    token_id: nft.token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        print!("{:?}", approvals);

        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: nft.nft.to_string(),
                token_id: nft.token_id.clone(),
            })],
            duration: None,
            max_tickets: None,
            min_ticket_number: None,
            gating: vec![],
        };

        create_raffle_setup(params).unwrap();

        // Purchasing tickets for 3 people
        // ensure error if max tickets per address set is reached
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![two.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![three.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let _purchase_tickets = buy_tickets_template(params).unwrap();
        let owner_balance_before = app.wrap().query_balance(&owner_addr, "ustars").unwrap();

        finish_raffle_timeout(
            &mut app,
            &contracts,
            0,
            MINIMUM_RAFFLE_DURATION + NOIS_TIMEOUT,
        )
        .unwrap();

        // queries the raffle
        let res = raffle_info(&app, &contracts, 0);

        // verify winner is always owner
        assert_eq!(
            two,
            res.raffle_info.unwrap().winners[0],
            "winner should always be the owner if no tickets were bought"
        );
        // verify no tickets can be bought after raffle ends
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 1,
            funds_send: vec![coin(4, "ustars")],
        };
        // simulate the puchase of tickets
        let purchase_tickets = buy_tickets_template(params);
        assert!(
            purchase_tickets.is_err(),
            "There should be an issue with purchasing a ticket once the winner is determined"
        );

        // We make sure the owner has more balance
        let owner_balance_after = app.wrap().query_balance(&owner_addr, "ustars").unwrap();

        assert_eq!(
            owner_balance_before.amount
                + (Decimal::percent(100) - Decimal::percent(50))
                    * (Uint128::from(4u128) * Uint128::from(3u128)), // 50% fee fee for everyone
            owner_balance_after.amount
        );
    }

    mod query {
        use cw_multi_test::{BankSudo, SudoMsg};
        use raffles::msg::FeeDiscountResponse;

        use super::*;

        #[test]
        pub fn owner_has_nft() {
            let (mut app, contracts) = proper_raffle_instantiate(None, OWNER_ADDR.to_string());
            let (owner_addr, _, _) = setup_accounts(&mut app);

            let user_discount: FeeDiscountResponse = app
                .wrap()
                .query_wasm_smart(
                    contracts.raffle,
                    &raffles::msg::QueryMsg::FeeDiscount {
                        user: owner_addr.to_string(),
                    },
                )
                .unwrap();

            assert_eq!(user_discount.discounts.len(), 2);
            assert!(user_discount.discounts[0].1);
            assert!(!user_discount.discounts[1].1);
            assert_eq!(user_discount.total_discount, Decimal::one())
        }

        #[test]
        pub fn owner_has_nft_and_staker() {
            let (mut app, contracts) = proper_raffle_instantiate(None, OWNER_ADDR.to_string());
            let (owner_addr, _, _) = setup_accounts(&mut app);

            app.execute(
                owner_addr.clone(),
                cosmwasm_std::CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate {
                    validator: "validator".to_string(),
                    amount: coin(150, "TOKEN"),
                }),
            )
            .unwrap();

            let user_discount: FeeDiscountResponse = app
                .wrap()
                .query_wasm_smart(
                    contracts.raffle,
                    &raffles::msg::QueryMsg::FeeDiscount {
                        user: owner_addr.to_string(),
                    },
                )
                .unwrap();

            assert_eq!(user_discount.discounts.len(), 2);
            assert!(user_discount.discounts[0].1);
            assert!(user_discount.discounts[1].1);
            assert_eq!(user_discount.total_discount, Decimal::one())
        }

        #[test]
        pub fn owner_has_staker() {
            let (mut app, contracts) = proper_raffle_instantiate(None, OWNER_ADDR.to_string());
            let (_, _, _) = setup_accounts(&mut app);
            let (one, _, _, _, _, _) = setup_raffle_participants(&mut app);

            app.sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: one.to_string(),
                    amount: vec![
                        coin(150, "TOKEN"), // For staking
                    ],
                }
            }))
            .unwrap();
            app.execute(
                one.clone(),
                cosmwasm_std::CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate {
                    validator: "validator".to_string(),
                    amount: coin(150, "TOKEN"),
                }),
            )
            .unwrap();

            let user_discount: FeeDiscountResponse = app
                .wrap()
                .query_wasm_smart(
                    contracts.raffle,
                    &raffles::msg::QueryMsg::FeeDiscount {
                        user: one.to_string(),
                    },
                )
                .unwrap();

            assert_eq!(user_discount.discounts.len(), 2);
            assert!(!user_discount.discounts[0].1);
            assert!(user_discount.discounts[1].1);
            assert_eq!(user_discount.total_discount, Decimal::percent(50))
        }
    }
}
