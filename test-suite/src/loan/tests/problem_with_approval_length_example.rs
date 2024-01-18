#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, BlockInfo, Coin, Decimal, Empty, Timestamp, Uint128};
    use cw_multi_test::{BankSudo, Executor, SudoMsg};
    use nft_loans::{
        msg::{ExecuteMsg, InstantiateMsg},
        state::{Config, LoanTerms},
    };
    use sg721::CollectionInfo;
    use sg_multi_test::StargazeApp;
    use sg_std::NATIVE_DENOM;
    use utils::state::{AssetInfo, Sg721Token};
    use vending_factory::{
        msg::VendingMinterCreateMsg,
        state::{ParamsExtension, VendingMinterParams},
    };

    use crate::common_setup::contract_boxes::{
        contract_nft_loans, contract_sg721_base, contract_vending_factory, contract_vending_minter,
        custom_mock_app,
    };

    const OWNER_ADDR: &str = "fee";
    const OFFERER_ADDR: &str = "offerer";
    const VENDING_MINTER: &str = "contract2";
    const SG721_CONTRACT: &str = "contract3";

    pub fn proper_instantiate() -> (StargazeApp, Addr, Addr) {
        // setup mock blockchain environment
        let mut app = custom_mock_app();
        let chainid = app.block_info().chain_id.clone();
        app.set_block(BlockInfo {
            height: 10000,
            time: Timestamp::from_nanos(1647032400000000000),
            chain_id: chainid,
        });

        // fund test account
        app.sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: OWNER_ADDR.to_string(),
                amount: vec![coin(100000000000u128, NATIVE_DENOM.to_string())],
            }
        }))
            .unwrap();
        app.sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: OFFERER_ADDR.to_string(),
                amount: vec![coin(100000000000u128, NATIVE_DENOM.to_string())],
            }
        }))
            .unwrap();

        // store wasm code for nft, minter , nft-loan
        let loan_code_id = app.store_code(contract_nft_loans());
        let factory_id = app.store_code(contract_vending_factory());
        let minter_id = app.store_code(contract_vending_minter());
        let sg721_id = app.store_code(contract_sg721_base());

        // setup nft minter
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
        // create nft-loan contract
        let nft_loan_addr = app
            .instantiate_contract(
                loan_code_id,
                Addr::unchecked(OWNER_ADDR),
                &InstantiateMsg {
                    name: "loan-with-insights".to_string(),
                    owner: Some(Addr::unchecked(OWNER_ADDR).to_string()),
                    fee_distributor: Addr::unchecked(OWNER_ADDR).to_string(),
                    fee_rate: Decimal::percent(5),
                    deposit_fee_denom: vec!["usstars".to_string(), NATIVE_DENOM.to_string()],
                    deposit_fee_amount: 50,
                },
                &[],
                "loans",
                Some(Addr::unchecked(OWNER_ADDR).to_string()),
            )
            .unwrap();

        (app, nft_loan_addr, factory_addr)
    }

    // certain functions may impact other functions, if present during testing.
    // TODO: update test workflow to prevent this issue
    mod init {
        use cosmwasm_std::StdError;
        use cw_utils::Expiration;
        use nft_loans::{
            error::ContractError,
            msg::{CollateralResponse, MultipleCollateralsResponse, OfferResponse, QueryMsg},
            state::{CollateralInfo, LoanState, OfferState},
        };

        use crate::common_setup::helpers::assert_error;

        use super::*;

        #[test]
        fn can_init() {
            let (mut app, nft_loan_addr, factory_addr) = proper_instantiate();

            let current_time = app.block_info().time.clone();
            let current_block = app.block_info().height.clone();

            // query contract config
            let query_config: Config = app
                .wrap()
                .query_wasm_smart(nft_loan_addr.clone(), &nft_loans::msg::QueryMsg::Config {})
                .unwrap();
            assert_eq!(query_config.owner, Addr::unchecked("fee"));

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
            let mint_nft_tokens = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    Addr::unchecked(VENDING_MINTER),
                    &vending_minter::msg::ExecuteMsg::Mint {},
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100000u128),
                    }],
                );
            assert!(mint_nft_tokens.is_ok());

            // token id 41
            let grant_approval = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    Addr::unchecked(SG721_CONTRACT),
                    &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                        spender: nft_loan_addr.to_string(),
                        token_id: "41".to_string(),
                        expires: Some(Expiration::AtHeight(current_block + 1)),
                    },
                    &[],
                );
            assert!(grant_approval.is_ok());

            // good deposit single collateral
            let good_deposit_collateral = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    nft_loan_addr.clone(),
                    &ExecuteMsg::DepositCollaterals {
                        tokens: vec![AssetInfo::Sg721Token(Sg721Token {
                            address: SG721_CONTRACT.to_string(),
                            token_id: "41".to_string(),
                        })],
                        terms: Some(LoanTerms {
                            principle: Coin {
                                denom: NATIVE_DENOM.to_string(),
                                amount: Uint128::new(100),
                            },
                            interest: Uint128::new(50),
                            // duration > approval
                            duration_in_blocks: 15,
                        }),
                        comment: Some("be water, my friend".to_string()),
                        loan_preview: None,
                    },
                    &[Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100u128),
                    }],
                );
            assert!(good_deposit_collateral.is_ok());

            // then someone will accept the offer but approval is past due
            let chainid = app.block_info().chain_id.clone();
            app.set_block(BlockInfo {
                height: current_block.clone() + 20,
                time: current_time.clone().plus_seconds(100),
                chain_id: chainid.clone(),
            });
            let good_accept_loan = app
                .execute_contract(
                    Addr::unchecked(OWNER_ADDR),
                    nft_loan_addr.clone(),
                    &ExecuteMsg::AcceptLoan {
                        borrower: OWNER_ADDR.to_string(),
                        loan_id: 0,
                        comment: None,
                    },
                    &[Coin { denom: NATIVE_DENOM.to_string(), amount: Uint128::new(100u128) }]);
            println!("{:#?}", good_accept_loan);


        }
    }
}
