#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, Uint128};
    use cw_multi_test::{BankSudo, Executor, SudoMsg};
    use raffles::{
        execute::NOIS_TIMEOUT,
        msg::InstantiateMsg,
        state::{StakerFeeDiscount, MINIMUM_RAFFLE_DURATION},
    };
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};
    use vending_factory::state::{ParamsExtension, VendingMinterParams};

    use crate::{
        common_setup::{
            app::StargazeApp,
            contract_boxes::custom_mock_app,
            helpers::setup_block_time,
            msg::RaffleContracts,
            nois_proxy::{self, NOIS_AMOUNT, NOIS_DENOM},
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_minter::common::constants::{
                CREATION_FEE_AMNT_NATIVE, CREATION_FEE_AMNT_STARS, OWNER_ADDR, RAFFLE_NAME,
                TREASURY_ADDR,
            },
            setup_raffle::raffle_template_code_ids,
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
        let mut app = custom_mock_app();
        let chainid = app.block_info().chain_id.clone();
        setup_block_time(&mut app, 1647032400000000000, Some(10000), &chainid);
        setup_accounts(&mut app);

        let code_ids = raffle_template_code_ids(&mut app);

        // TODO: setup_factory_template
        let factory_addr = app
            .instantiate_contract(
                code_ids.factory_code_id,
                Addr::unchecked(OWNER_ADDR),
                &vending_factory::msg::InstantiateMsg {
                    params: VendingMinterParams {
                        code_id: code_ids.minter_code_id,
                        allowed_sg721_code_ids: vec![code_ids.sg721_code_id],
                        frozen: false,
                        creation_fee: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100000u128),
                        },
                        min_mint_price: Coin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: Uint128::new(100000u128),
                        },
                        mint_fee_bps: 10,
                        max_trading_offset_secs: 0,
                        extension: ParamsExtension {
                            max_token_limit: 1000,
                            max_per_address_limit: 20,
                            airdrop_mint_price: Coin {
                                denom: NATIVE_DENOM.to_string(),
                                amount: Uint128::new(100000u128),
                            },
                            airdrop_mint_fee_bps: 10,
                            shuffle_fee: Coin {
                                denom: NATIVE_DENOM.to_string(),
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

        // Create the nois contract
        let nois_addr = app
            .instantiate_contract(
                code_ids.nois_code_id,
                Addr::unchecked(OWNER_ADDR),
                &nois_proxy::InstantiateMsg {
                    nois: NOIS_DENOM.to_string(),
                },
                &[],
                "nois-contract",
                None,
            )
            .unwrap();

        let nft = mint_one_token(
            &mut app,
            &RaffleContracts {
                factory: factory_addr.clone(),
                raffle: factory_addr.clone(),
                nois: nois_addr.clone(),
            },
        );

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

        // create raffle contract
        let raffle_contract_addr = app
            .instantiate_contract(
                code_ids.raffle_code_id,
                Addr::unchecked(OWNER_ADDR),
                &InstantiateMsg {
                    name: RAFFLE_NAME.to_string(),
                    nois_proxy_addr: nois_addr.to_string(),
                    nois_proxy_coin: coin(NOIS_AMOUNT, NOIS_DENOM.to_string()),
                    owner: Some(OWNER_ADDR.to_string()),
                    fee_addr: Some(TREASURY_ADDR.to_owned()),
                    minimum_raffle_duration: None,
                    max_ticket_number,
                    raffle_fee: Decimal::percent(50),
                    creation_coins: vec![
                        coin(CREATION_FEE_AMNT_NATIVE, NATIVE_DENOM.to_string()),
                        coin(CREATION_FEE_AMNT_STARS, "ustars".to_string()),
                    ]
                    .into(),
                    atlas_dao_nft_addresses: vec![nft.nft.to_string()],
                    staker_fee_discount: StakerFeeDiscount {
                        discount: Decimal::percent(50),
                        minimum_amount: Uint128::from(100u128),
                    },
                },
                &[],
                "raffle",
                Some(Addr::unchecked(OWNER_ADDR).to_string()),
            )
            .unwrap();

        // fund raffle contract for nois_proxy fee
        app.sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: raffle_contract_addr.clone().to_string(),
                amount: vec![coin(100000000000u128, NOIS_DENOM.to_string())],
            }
        }))
        .unwrap();

        (
            app,
            RaffleContracts {
                factory: factory_addr,
                raffle: raffle_contract_addr,
                nois: nois_addr,
            },
        )
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
}
