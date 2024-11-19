#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Binary, Decimal, Uint128};
    use cw_multi_test::Executor;
    use raffles::{
        error::ContractError,
        msg::{ConfigResponse, DrandConfig, ExecuteMsg, QueryMsg as RaffleQueryMsg},
    };
    use rustc_serialize::hex::FromHex;
    use utils::state::{AssetInfo, Locks, Sg721Token, SudoMsg as RaffleSudoMsg, NATIVE_DENOM};

    use crate::{
        common_setup::{
            app::StargazeApp,
            helpers::assert_error,
            msg::RaffleContracts,
            setup_accounts_and_block::setup_accounts,
            setup_minter::common::constants::{
                CREATION_FEE_AMNT_NATIVE, CREATION_FEE_AMNT_STARS, OWNER_ADDR, RAFFLE_NAME,
                RAFFLE_TAX, TREASURY_ADDR,
            },
            setup_raffle::{proper_raffle_instantiate, DRAND_TIMEOUT, DRAND_URL, HEX_PUBKEY},
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_function},
            helpers::{mint_one_token, TokenMint},
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };

    fn create_raffle(
        app: &mut StargazeApp,
        contracts: &RaffleContracts,
        token: &TokenMint,
    ) -> anyhow::Result<()> {
        let create_raffle_params = CreateRaffleParams {
            app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: Addr::unchecked(OWNER_ADDR),
            creation_fee: vec![coin(CREATION_FEE_AMNT_STARS, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id.clone(),
            })],
            duration: None,
            min_ticket_number: None,
            max_tickets: None,
            gating: vec![],
        };
        create_raffle_function(create_raffle_params)?;

        Ok(())
    }

    #[test]
    fn test_raffle_config_query() {
        let (app, contracts) = proper_raffle_instantiate();

        let query_config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contracts.raffle.clone(), &RaffleQueryMsg::Config {})
            .unwrap();
        assert_eq!(
            query_config,
            ConfigResponse {
                name: RAFFLE_NAME.into(),
                owner: OWNER_ADDR.to_string(),
                fee_addr: TREASURY_ADDR.to_string(),
                last_raffle_id: 0,
                minimum_raffle_duration: 1,
                max_tickets_per_raffle: Some(100_000),
                raffle_fee: RAFFLE_TAX,
                creation_coins: vec![
                    coin(CREATION_FEE_AMNT_NATIVE, NATIVE_DENOM),
                    coin(CREATION_FEE_AMNT_STARS, NATIVE_DENOM)
                ],
                locks: Locks {
                    lock: false,
                    sudo_lock: false,
                },
                fee_discounts: vec![],
                drand_config: DrandConfig {
                    random_pubkey: Binary::from(HEX_PUBKEY.from_hex().unwrap()),
                    drand_url: DRAND_URL.to_string(),
                    verify_signature_contract: contracts.randomness_verifier.clone(),
                    timeout: DRAND_TIMEOUT
                },
            }
        )
    }

    #[test]
    fn test_raffle_contract_config_permissions_coverage() {
        let (mut app, contracts) = proper_raffle_instantiate();
        // errors
        // unable to update contract config
        let error_updating_config = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                contracts.raffle.clone(),
                &ExecuteMsg::UpdateConfig {
                    name: Some("not-owner".to_string()),
                    owner: None,
                    fee_addr: None,
                    minimum_raffle_duration: None,
                    raffle_fee: None,
                    creation_coins: None,
                    max_tickets_per_raffle: None,
                    fee_discounts: None,
                    drand_config: None,
                },
                &[],
            )
            .unwrap_err();
        // unable to lock contract
        let error_locking_contract = app
            .execute_contract(
                Addr::unchecked("not-owner"),
                contracts.raffle.clone(),
                &raffles::msg::ExecuteMsg::ToggleLock { lock: true },
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
        let _updating_config = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                contracts.raffle.clone(),
                &ExecuteMsg::UpdateConfig {
                    name: Some("new-owner".to_string()),
                    owner: Some("new-owner".to_string()),
                    fee_addr: Some("new-owner".to_string()),
                    minimum_raffle_duration: Some(60),
                    raffle_fee: Some(Decimal::percent(99)),
                    creation_coins: Some(vec![coin(420, "new-new")]),
                    max_tickets_per_raffle: None,
                    fee_discounts: None,
                    drand_config: None,
                },
                &[],
            )
            .unwrap();
        // good responses
        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contracts.raffle.clone(), &RaffleQueryMsg::Config {})
            .unwrap();
        assert_eq!(
            res,
            ConfigResponse {
                name: "new-owner".to_string(),
                owner: "new-owner".to_string(),
                fee_addr: "new-owner".to_string(),
                last_raffle_id: 0,
                minimum_raffle_duration: 60,
                max_tickets_per_raffle: Some(100_000),
                raffle_fee: Decimal::percent(99),
                creation_coins: vec![coin(420, "new-new")],
                locks: Locks {
                    lock: false,
                    sudo_lock: false,
                },
                fee_discounts: vec![],
                drand_config: DrandConfig {
                    random_pubkey: Binary::from(HEX_PUBKEY.from_hex().unwrap()),
                    drand_url: DRAND_URL.to_string(),
                    verify_signature_contract: contracts.randomness_verifier.clone(),
                    timeout: DRAND_TIMEOUT
                },
            }
        )
    }

    #[test]
    fn good_toggle_lock() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_address, one, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        create_raffle(&mut app, &contracts, &token).unwrap();

        let _invalid_toggle_lock = app
            .execute_contract(
                owner_address.clone(),
                contracts.raffle.clone(),
                &ExecuteMsg::ToggleLock { lock: true },
                &[],
            )
            .unwrap();
        // confirm the state is now true
        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contracts.raffle.to_string(), &RaffleQueryMsg::Config {})
            .unwrap();
        assert!(res.locks.lock);

        let locked_creation = create_raffle(&mut app, &contracts, &token).unwrap_err();
        assert_error(
            Err(locked_creation),
            ContractError::ContractIsLocked {}.to_string(),
        );

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
        assert!(purchase_tickets.is_ok());
    }

    #[test]
    fn good_toggle_sudo_lock() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (_, one, _) = setup_accounts(&mut app);
        let token = mint_one_token(&mut app, &contracts);

        create_raffle(&mut app, &contracts, &token).unwrap();

        let _invalid_toggle_lock = app
            .wasm_sudo(
                contracts.raffle.clone(),
                &RaffleSudoMsg::ToggleLock { lock: true },
            )
            .unwrap();

        // confirm the state is now true
        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(contracts.raffle.to_string(), &RaffleQueryMsg::Config {})
            .unwrap();
        assert!(res.locks.sudo_lock);

        let locked_creation = create_raffle(&mut app, &contracts, &token).unwrap_err();
        assert_error(
            Err(locked_creation),
            ContractError::ContractIsLocked {}.to_string(),
        );

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
        assert!(purchase_tickets.is_ok());
    }
}
