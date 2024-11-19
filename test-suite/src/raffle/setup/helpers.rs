use crate::common_setup::app::StargazeApp;
use crate::common_setup::{msg::RaffleContracts, setup_minter::common::constants::OWNER_ADDR};
use cosmwasm_std::{coin, Addr, Binary, BlockInfo, Coin, Empty, Uint128};
use cw_multi_test::Executor;
use raffles::msg::{ExecuteMsg, QueryMsg, RaffleResponse};
use randomness::DrandRandomness;
use rustc_serialize::hex::FromHex;
use sg721::CollectionInfo;
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

pub fn finish_raffle_timeout_generic(
    app: &mut StargazeApp,
    contracts: &RaffleContracts,
    raffle_id: u64,
    timeout: u64,
    randomness_id: u8,
) -> anyhow::Result<()> {
    // We advance time to be able to send the nois message
    let block = app.block_info();
    app.set_block(BlockInfo {
        height: block.height,
        time: block.time.plus_seconds(timeout),
        chain_id: block.chain_id,
    });

    // We send a randomness message, to update it
    send_update_randomness_message(app, contracts, raffle_id, randomness_id)?;

    app.execute_contract(
        contracts.raffle.clone(),
        contracts.raffle.clone(),
        &ExecuteMsg::ClaimRaffle { raffle_id },
        &[],
    )?;

    Ok(())
}
pub fn finish_raffle_timeout(
    app: &mut StargazeApp,
    contracts: &RaffleContracts,
    raffle_id: u64,
    timeout: u64,
) -> anyhow::Result<()> {
    finish_raffle_timeout_generic(app, contracts, raffle_id, timeout, 0)
}
pub struct DrandRandomnessConst<'a> {
    pub round: u64,
    pub previous_signature: &'a str,
    pub signature: &'a str,
}

pub const RANDOMNESS_1: DrandRandomnessConst = DrandRandomnessConst{
    round: 4552804u64,
    previous_signature: "90cf2fb5a6b126d0b42e1c1446f23d3dff4986fbe99b10d88f822432e672586ee9cc56ec8174b849f2f6c9d3774ed2380f77d9cb4239bc1a1e00b30933bb22d758d3ef1a148e219676ff17081f4ab21ea4debe58707611bcc8a2486b7ed8b36f",
    signature: "97e13842ed58f082e0e3f264e55e99c3c233cd750a4c2b62473549c9cacba7a430433f0d9060a7bda42d45cc04d5544f1419bee036090d636e9573dea76f77b6583c2c9e5cd2322df9ba4cef9cdc4c11c08d0492b9ec787259aba276bf5a5fbf"
};

pub const RANDOMNESS_2: DrandRandomnessConst = DrandRandomnessConst{
    round: 4552808u64,
    previous_signature: "87b33c2f096ddb3c3b8697cf3b31da05e9e77fb1baf89fae2e527993e4570b79bb052a6279b02409559f0eb631d8b1f9130018a5d38ea7b7b45a907d611185fb99798ad8efcf52584a3291772e5d427c6356cb564c336c900f07599e0e0000d0",
    signature: "93e8f7e949548928a32a8d5084558832b81039fdee3fec19e4c2acfed8f88b5986c0ee5988b75154a90d58d2c47b168207164ba509a331dfd002e06d666bd5eaa50dddc23aebcc768b2fe6559aef7facdee75c9d42c5fd76527de7d74da0de1c"
};

pub fn send_update_randomness_message_generic(
    app: &mut StargazeApp,
    contracts: &RaffleContracts,
    raffle_id: u64,
    randomness: DrandRandomness,
) -> anyhow::Result<()> {
    // We send a nois message (that is automatic in the real world)
    app.execute_contract(
        Addr::unchecked("anyone, really"),
        contracts.raffle.clone(),
        &raffles::msg::ExecuteMsg::UpdateRandomness {
            raffle_id,
            randomness,
        },
        &[],
    )?;
    Ok(())
}

pub fn send_update_randomness_message(
    app: &mut StargazeApp,
    contracts: &RaffleContracts,
    raffle_id: u64,
    randomness_id: u8,
) -> anyhow::Result<()> {
    if randomness_id == 0 {
        send_update_randomness_message_generic(app, contracts, raffle_id, rand(RANDOMNESS_1)?)
    } else {
        send_update_randomness_message_generic(app, contracts, raffle_id, rand(RANDOMNESS_2)?)
    }
}

pub fn rand(rand: DrandRandomnessConst) -> anyhow::Result<DrandRandomness> {
    Ok(DrandRandomness {
        round: rand.round,
        previous_signature: Binary::from(rand.previous_signature.from_hex()?),
        signature: Binary::from(rand.signature.from_hex()?),
    })
}
