use crate::common_setup::nois_proxy::{ERROR_ON_NOIS_EXEC, TEST_NOIS_PREFIX};
use crate::common_setup::{msg::RaffleContracts, setup_minter::common::constants::OWNER_ADDR};
use anyhow::bail;
use cosmwasm_std::{coin, Addr, BlockInfo, Coin, Empty, Uint128};
use cw_multi_test::Executor;
use nois::ProxyExecuteMsg;
use raffles::msg::{ExecuteMsg, QueryMsg, RaffleResponse};
use sg721::CollectionInfo;
use sg_multi_test::StargazeApp;
use utils::state::NATIVE_DENOM;
use vending_factory::msg::VendingMinterCreateMsg;

pub fn raffle_info(
    app: &StargazeApp,
    contracts: &RaffleContracts,
    raffle_id: u64,
) -> RaffleResponse {
    app.wrap()
        .query_wasm_smart(
            contracts.raffle.clone(),
            &QueryMsg::RaffleInfo { raffle_id },
        )
        .unwrap()
}

pub struct TokenMint {
    pub minter: Addr,
    pub nft: Addr,
    pub token_id: String,
}

pub fn mint_one_token(app: &mut StargazeApp, contracts: &RaffleContracts) -> TokenMint {
    let current_block = app.block_info();
    // create nft minter
    let create_nft_minter = app.execute_contract(
        Addr::unchecked(OWNER_ADDR),
        contracts.factory.clone(),
        &vending_factory::msg::ExecuteMsg::CreateMinter(VendingMinterCreateMsg {
            init_msg: vending_factory::msg::VendingMinterInitMsgExtension {
                base_token_uri: "ipfs://aldkfjads".to_string(),
                payment_address: Some(OWNER_ADDR.to_string()),
                start_time: current_block.time,
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
        }),
        &[Coin {
            denom: NATIVE_DENOM.to_string(),
            amount: Uint128::new(100000u128),
        }],
    );
    let addresses = create_nft_minter
        .unwrap()
        .events
        .into_iter()
        .filter(|e| e.ty == "instantiate")
        .flat_map(|e| {
            e.attributes
                .into_iter()
                .filter(|a| a.key == "_contract_addr")
                .map(|a| a.value)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let minter_address = addresses[0].clone();
    let nft_address = addresses[1].clone();

    // VENDING_MINTER is minter
    let mint_nft_tokens = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            Addr::unchecked(minter_address.clone()),
            &vending_minter::msg::ExecuteMsg::Mint {},
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100000u128),
            }],
        )
        .unwrap();

    let token_id = mint_nft_tokens
        .events
        .into_iter()
        .filter(|e| e.ty == "wasm")
        .flat_map(|e| {
            e.attributes
                .into_iter()
                .filter(|a| a.key == "token_id")
                .map(|a| a.value)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()[0]
        .clone();

    let _grant_approval = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            Addr::unchecked(nft_address.clone()),
            &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                spender: contracts.raffle.to_string(),
                token_id: token_id.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

    TokenMint {
        minter: Addr::unchecked(minter_address),
        nft: Addr::unchecked(nft_address),
        token_id,
    }
}

pub fn mint_additional_token(
    app: &mut StargazeApp,
    contracts: &RaffleContracts,
    token: &TokenMint,
) -> TokenMint {
    // VENDING_MINTER is minter
    let mint_nft_tokens = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            token.minter.clone(),
            &vending_minter::msg::ExecuteMsg::Mint {},
            &[Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(100000u128),
            }],
        )
        .unwrap();

    let token_id = mint_nft_tokens
        .events
        .into_iter()
        .filter(|e| e.ty == "wasm")
        .flat_map(|e| {
            e.attributes
                .into_iter()
                .filter(|a| a.key == "token_id")
                .map(|a| a.value)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()[0]
        .clone();

    let _grant_approval = app
        .execute_contract(
            Addr::unchecked(OWNER_ADDR),
            Addr::unchecked(token.nft.clone()),
            &sg721_base::msg::ExecuteMsg::<Empty, Empty>::Approve {
                spender: contracts.raffle.to_string(),
                token_id: token_id.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

    TokenMint {
        minter: token.minter.clone(),
        nft: token.nft.clone(),
        token_id,
    }
}

pub fn finish_raffle_timeout(
    app: &mut StargazeApp,
    contracts: &RaffleContracts,
    raffle_id: u64,
    timeout: u64,
) -> anyhow::Result<()> {
    // We advance time to be able to send the nois message
    let block = app.block_info();
    app.set_block(BlockInfo {
        height: block.height,
        time: block.time.plus_seconds(timeout),
        chain_id: block.chain_id,
    });

    // We send a nois message (that is automatic in the real world)
    send_nois_ibc_message(app, contracts, raffle_id)?;
    app.execute_contract(
        contracts.raffle.clone(),
        contracts.raffle.clone(),
        &ExecuteMsg::ClaimRaffle { raffle_id },
        &[],
    )?;

    Ok(())
}

pub fn send_nois_ibc_message(
    app: &mut StargazeApp,
    contracts: &RaffleContracts,
    raffle_id: u64,
) -> anyhow::Result<()> {
    // We send a nois message (that is automatic in the real world)
    let response = app.execute_contract(
        contracts.raffle.clone(),
        contracts.nois.clone(),
        &ProxyExecuteMsg::GetNextRandomness {
            job_id: format!("{TEST_NOIS_PREFIX}raffle-{raffle_id}"),
        },
        &[],
    )?;
    // We get the error value
    let error_value = response
        .events
        .into_iter()
        .filter(|e| e.ty == "wasm")
        .flat_map(|e| {
            e.attributes
                .into_iter()
                .filter(|a| a.key == ERROR_ON_NOIS_EXEC)
                .map(|a| a.value)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    if error_value.is_empty() {
        Ok(())
    } else {
        bail!("Error on executing on nois randomnesss : {:?}", error_value)
    }
}
