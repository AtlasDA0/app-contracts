use std::error::Error;

use abstract_cw_multi_test::{Contract, IntoAddr};
use cw_infuser::{
    msg::{ExecuteMsgFns, InstantiateMsg, QueryMsgFns},
    state::{
        Bundle, BurnParams, Config, InfusedCollection, Infusion, InfusionParams, NFTCollection, NFT,
    },
};
// Use prelude to get all the necessary imports
use cw_orch::{anyhow, prelude::*};
use scripts::CwInfuser;

fn cw721_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}
// minimal infuser
pub struct InfuserSuite<Chain> {
    pub chain: MockBech32,
    pub infuser: CwInfuser<Chain>,
    pub nfts: Vec<Addr>,
    
}

impl<Chain: CwEnv> InfuserSuite<Chain> {
    fn setup() -> anyhow::Result<InfuserSuite<MockBech32>> {
        let mock = MockBech32::new("mock");
        let sender = mock.sender_addr();
        let infuser = CwInfuser::new(mock.clone());
        infuser.upload()?;

        // store cw721
        let cw721 = cw721_contract();
        let cw721_code_id = mock.upload_custom("cw721", cw721)?.uploaded_code_id()?;

        // instanatiate cw721
        let msg_a = mock.instantiate(
            cw721_code_id,
            &cw721_base::InstantiateMsg {
                name: "good-chronic".to_string(),
                symbol: "CHRONIC".to_string(),
                minter: Some(sender.to_string()),
                withdraw_address: Some(sender.to_string()),
            },
            Some("cw721-base-good-chronic"),
            None,
            &[],
        )?;
        let cw721_a = msg_a.instantiated_contract_address()?;

        // mint 11 nfts?
        for n in 0..10 {
            // mint cw721
            mock.execute(
                &cw721_base::ExecuteMsg::<Option<Empty>, Empty>::Mint {
                    token_id: n.to_string(),
                    owner: sender.to_string(),
                    token_uri: None,
                    extension: None,
                },
                &[],
                &cw721_a.clone(),
            )?;
        }

        // create cw-infsion app
        infuser.instantiate(
            &InstantiateMsg {
                admin: Some(sender.to_string()),
                max_bundles: None,
                max_infusions: None,
                max_token_in_bundle: None,
                cw721_code_id,
            },
            None,
            None,
        )?;

        for n in 0..10 {
            // approve infuser for nft
            mock.execute(
                &cw721_base::ExecuteMsg::<Option<Empty>, Empty>::Approve {
                    spender: infuser.address()?.to_string(),
                    token_id: n.to_string(),
                    expires: None,
                },
                &[],
                &cw721_a.clone(),
            )?;
        }

        // create infusion
        infuser.create_infusion(vec![Infusion {
            collections: vec![NFTCollection {
                addr: cw721_a.clone(),
                min_wanted: 2,
                max: Some(2),
            }],
            infused_collection: InfusedCollection {
                addr: Addr::unchecked("test"),
                admin: None,
                name: "test".to_string(),
                symbol: "TEST".to_string(),
            },
            infusion_params: InfusionParams {
                params: BurnParams {
                    compatible_traits: None,
                },
            },
            infusion_id: 1,
        }])?;

        Ok(InfuserSuite {
            chain: mock,
            infuser,
            nfts: vec![cw721_a],
        })
    }
}

#[test]
fn successful_install() -> anyhow::Result<()> {
    let env = InfuserSuite::<MockBech32>::setup()?;
    let app = env.infuser;

    let config = app.config()?;
    assert_eq!(
        config,
        Config {
            latest_infusion_id: Some(0),
            admin: env.chain.sender_addr(),
            max_infusions: 2u64,
            min_per_bundle: 1u64,
            max_per_bundle: 10u64,
            max_bundles: 5u64,
            code_id: 2,
        }
    );
    Ok(())
}

#[test]
fn successful_infusion() -> anyhow::Result<()> {
    let env = InfuserSuite::<MockBech32>::setup()?;
    let app = env.infuser;
    let sender = env.chain.sender_addr();

    // create first infusion.
    app.infuse(
        vec![Bundle {
            nfts: vec![
                NFT {
                    addr: env.nfts[0].clone(),
                    token_id: 1,
                },
                NFT {
                    addr: env.nfts[0].clone(),
                    token_id: 3,
                },
            ],
        }],
        0,
    )?;
    // confirm infused collection mint
    let res = app.infused_collection(0)?;
    assert_eq!(res.symbol, "TEST");

    // error if too few nfts provided in bundle
    let err = app
        .infuse(
            vec![Bundle {
                nfts: vec![NFT {
                    addr: env.nfts[0].clone(),
                    token_id: 2,
                }],
            }],
            0,
        )
        .unwrap_err();

    assert_eq!(
        err.source().unwrap().to_string(),
        "Not enough nfts in bundle collection mock1uzyszmsnca8euusre35wuqj4el3hyj8jty84kwln7du5stwwxynsm6wxnq"
    );

    // error if too many nfts provided in bundle
    let err = app
        .infuse(
            vec![Bundle {
                nfts: vec![
                    NFT {
                        addr: env.nfts[0].clone(),
                        token_id: 2,
                    },
                    NFT {
                        addr: env.nfts[0].clone(),
                        token_id: 4,
                    },
                    NFT {
                        addr: env.nfts[0].clone(),
                        token_id: 6,
                    },
                ],
            }],
            0,
        )
        .unwrap_err();
    assert_eq!(err.source().unwrap().to_string(), "Too many nfts in bundle");

    // assert queries
    let res = app.infusion_by_id(0)?;
    assert_eq!(res.collections.len(), 1);
    assert_eq!(res.collections[0].min_wanted, 2);
    assert_eq!(res.infusion_id, 0);
    assert!(app.is_in_bundle(env.nfts[0].clone(), 0u64)?);
    assert!(!app.is_in_bundle(
        "mock1oqklo6g7ca7euusre35wuqj4el3hyj8jty84kwln7du5stwwxyns6h6h3f".into_addr(),
        0u64
    )?);
    Ok(())
}

// Multiple Collections In Bundle

// Correct Trait Requirement Logic

// Correct Fees & Destination
