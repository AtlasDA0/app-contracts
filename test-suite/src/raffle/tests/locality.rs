#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, to_json_binary, Addr, BlockInfo, Coin, HexBinary, Uint128};
    use cw721::NumTokensResponse;
    use cw_multi_test::{BankSudo, SudoMsg};
    use cw_multi_test::{Executor, WasmSudo};

    use nois::NoisCallback;
    use raffles::msg::{CollectionParams, CreateLocalityParams, LocalityMinterParams};

    use raffles::msg::ExecuteMsg;

    use sg721::{CollectionInfo, RoyaltyInfoResponse};
    use utils::state::AssetInfo;

    use crate::common_setup::contract_boxes::contract_sg721_base;

    use crate::common_setup::nois_proxy::{self, DEFAULT_RANDOMNESS_SEED};
    use crate::common_setup::setup_minter::common::constants::{OWNER_ADDR, RAFFLE_NAME};
    use crate::common_setup::setup_raffle::proper_raffle_instantiate_precise;
    use crate::raffle::setup::helpers::mint_one_token;

    #[test]
    fn create_single_locality() {
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
        // get block information
        let block_info = app.block_info();
        // create locality-instance
        let _good_create_locality = app
            .execute_contract(
                Addr::unchecked(OWNER_ADDR),
                contracts.raffle.clone(),
                &ExecuteMsg::CreateLocality {
                    locality_params: CreateLocalityParams {
                        init_msg: LocalityMinterParams {
                            mint_price: Coin {
                                denom: "ustars".to_string(),
                                amount: Uint128::from(25u64),
                            },
                            per_address_limit: Some(2),
                            num_tokens: 2,
                            duration: 60u64,
                            frequency: 2u64,
                            start_time: block_info.time,
                            harmonics: 1,
                            payment_address: None,
                            max_tickets: None,
                        },
                        collection_params: CollectionParams {
                            code_id: contracts.cw721.id,
                            name: "LOCALITY".to_string(),
                            symbol: "LOCAL".to_string(),
                            info: CollectionInfo::<RoyaltyInfoResponse> {
                                creator: Addr::unchecked(OWNER_ADDR).to_string(),
                                description: String::from("test"),
                                image: String::from("https://test.network"),
                                external_link: None,
                                explicit_content: None,
                                start_trading_time: None,
                                royalty_info: None,
                            },
                        },
                    },
                },
                &[],
            )
            .unwrap();

        // not in phase
        let current_time = app.block_info().time;
        app.set_block(BlockInfo {
            height: current_block + 1,
            time: current_time.plus_seconds(1),
            chain_id: chainid.clone(),
        });
        // not in phase
        let response: Vec<String> = app
            .wrap()
            .query_wasm_smart(
                contracts.raffle.clone(),
                &raffles::msg::QueryMsg::AllTickets {
                    raffle_id: 0,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        println!("{:#?}", response);
        // verify no nfts were minted
        let res: NumTokensResponse = app
            .wrap()
            .query_wasm_smart("contract5".to_string(), &cw721::Cw721QueryMsg::NumTokens {})
            .unwrap();
        assert_eq!(res.count, 0);

        // purchase 2 tickets
        let purchase = app
            .execute_contract(
                Addr::unchecked("wallet-1"),
                contracts.raffle.clone(),
                &ExecuteMsg::PurchaseLocalityTicket {
                    id: 0,
                    ticket_count: 2,
                    assets: AssetInfo::Coin(Coin {
                        denom: "ustars".to_string(),
                        amount: Uint128::from(50u64),
                    }),
                },
                &vec![coin(50, "ustars")],
            )
            .unwrap();
        println!("{:#?}", purchase);

        // in phase
        let current_time = app.block_info().time;
        app.set_block(BlockInfo {
            height: current_block + 2,
            time: current_time.plus_seconds(1),
            chain_id: chainid.clone(),
        });

        // mimic res from nois
        let _nois_res = app
            .execute_contract(
                contracts.nois.clone(),
                contracts.raffle.clone(),
                &raffles::msg::ExecuteMsg::NoisReceive {
                    callback: NoisCallback {
                        job_id: "locality-0".to_string(),
                        published: block_info.time,
                        randomness: HexBinary::from_hex(DEFAULT_RANDOMNESS_SEED).unwrap(),
                    },
                },
                &vec![],
            )
            .unwrap();

        // mimic sudo msg from chain
        let res = app
            .sudo(SudoMsg::Wasm(WasmSudo {
                contract_addr: contracts.raffle.clone(),
                msg: to_json_binary(&raffles::msg::SudoMsg::BeginBlock {}).unwrap(),
            }))
            .unwrap();

        // past 1 phase
        let current_time = app.block_info().time;
        app.set_block(BlockInfo {
            height: current_block + 1,
            time: current_time.plus_seconds(1),
            chain_id: chainid.clone(),
        });

        // verify 2 nfts were minted
        // let res: NumTokensResponse = app
        //     .wrap()
        //     .query_wasm_smart("contract5".to_string(), &cw721::Cw721QueryMsg::NumTokens {})
        //     .unwrap();
        // assert_eq!(res.count, 2);

        // finish_locality_timeout(&mut app, &contracts, 0, 60).unwrap();
    }
}
