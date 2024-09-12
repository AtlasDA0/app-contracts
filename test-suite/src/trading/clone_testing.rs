use cosmwasm_std::{coin, coins, Coins, Decimal, Uint128};
use cw721_base::interface::{Cw721, ExecuteMsg};
use cw_orch::{prelude::*, tokio::runtime::Runtime};
use cw_orch_clone_testing::CloneTesting;
use p2p_trading::P2PTrading;
use p2p_trading_export::msg::{AddAssetAction, ExecuteMsgFns, InstantiateMsg};
use utils::state::{AssetInfo, Sg721Token};

use crate::trading::{
    COUNTER_ID, COUNTER_TRADER, FEE_AMOUNT, FEE_DENOM, FIRST_FUND_AMOUNT, FUND_FEE, GECKIES_ADDRESS, GECKIES_ID, NICOCO_FEE_AMOUNT, OWNER, SECOND_FUND_AMOUNT, SNS, SNS_ADDRESS
};

use super::STARGAZE_1;

#[test]
fn actual_nft() -> anyhow::Result<()> {
    let rt = Runtime::new()?;
    let mut chain = CloneTesting::new(&rt, STARGAZE_1)?;
    chain.set_sender(Addr::unchecked(OWNER));

    let treasury = chain.app.borrow().api().addr_make("treasury");

    let nft: Cw721<CloneTesting> = Cw721::new("geckies", chain.clone());
    nft.set_address(&Addr::unchecked(GECKIES_ADDRESS));

    let p2p = P2PTrading::new(chain.clone());
    p2p.upload()?;
    p2p.instantiate(
        &InstantiateMsg {
            name: "AtlasDaoTrading".to_string(),
            owner: None,
            accept_trade_fee: vec![
                coin(NICOCO_FEE_AMOUNT, "nicoco"),
                coin(FEE_AMOUNT, FEE_DENOM),
            ],
            fund_fee: FUND_FEE,
            treasury: treasury.to_string(),
        },
        None,
        None,
    )?;

    nft.execute(
        &ExecuteMsg::ApproveAll {
            operator: p2p.address()?.to_string(),
            expires: None,
        },
        None,
    )?;
    nft.call_as(&Addr::unchecked(COUNTER_TRADER)).execute(
        &ExecuteMsg::ApproveAll {
            operator: p2p.address()?.to_string(),
            expires: None,
        },
        None,
    )?;

    p2p.create_trade(None, None)?;
    p2p.add_asset(
        AddAssetAction::ToLastTrade {},
        AssetInfo::Sg721Token(Sg721Token {
            address: nft.address()?.to_string(),
            token_id: GECKIES_ID.to_string(),
        }),
        &[],
    )?;

    p2p.confirm_trade(None)?;
    let trade_id = 0;

    let counter_p2p = p2p.call_as(&Addr::unchecked(COUNTER_TRADER));

    counter_p2p.suggest_counter_trade(trade_id, None)?;
    counter_p2p.add_asset(
        AddAssetAction::ToLastCounterTrade { trade_id },
        AssetInfo::Sg721Token(Sg721Token {
            address: nft.address()?.to_string(),
            token_id: COUNTER_ID.to_string(),
        }),
        &[],
    )?;

    counter_p2p.confirm_counter_trade(trade_id, None)?;

    let counter_id = 0;
    p2p.accept_trade(counter_id, trade_id, None)?;
    p2p.withdraw_successful_trade(trade_id, &coins(FEE_AMOUNT, FEE_DENOM))?;
    counter_p2p.withdraw_successful_trade(trade_id, &coins(FEE_AMOUNT, FEE_DENOM))?;

    let treasury_balance = chain.balance(treasury, Some(FEE_DENOM.to_string()))?;
    assert_eq!(treasury_balance, coins(2 * FEE_AMOUNT, FEE_DENOM));
    Ok(())
}

#[test]
fn sns() -> anyhow::Result<()> {
    let rt = Runtime::new()?;
    let mut chain = CloneTesting::new(&rt, STARGAZE_1)?;
    chain.set_sender(Addr::unchecked(OWNER));

    let nft: Cw721<CloneTesting> = Cw721::new("geckies", chain.clone());
    nft.set_address(&Addr::unchecked(GECKIES_ADDRESS));
    let sns = Cw721::new("sns", chain.clone());
    sns.set_address(&Addr::unchecked(SNS_ADDRESS));

    let p2p = P2PTrading::new(chain.clone());
    p2p.upload()?;

    let treasury = chain.app.borrow().api().addr_make("treasury");
    p2p.instantiate(
        &InstantiateMsg {
            name: "AtlasDaoTrading".to_string(),
            owner: None,
            accept_trade_fee: vec![
                coin(NICOCO_FEE_AMOUNT, "nicoco"),
                coin(FEE_AMOUNT, FEE_DENOM),
            ],
            fund_fee: FUND_FEE,
            treasury: treasury.to_string(),
        },
        None,
        None,
    )?;

    sns.execute(
        &ExecuteMsg::ApproveAll {
            operator: p2p.address()?.to_string(),
            expires: None,
        },
        None,
    )?;
    nft.call_as(&Addr::unchecked(COUNTER_TRADER)).execute(
        &ExecuteMsg::ApproveAll {
            operator: p2p.address()?.to_string(),
            expires: None,
        },
        None,
    )?;

    p2p.create_trade(None, None)?;
    p2p.add_asset(
        AddAssetAction::ToLastTrade {},
        AssetInfo::Sg721Token(Sg721Token {
            address: sns.address()?.to_string(),
            token_id: SNS.to_string(),
        }),
        &[],
    )?;

    p2p.confirm_trade(None)?;
    let trade_id = 0;

    let counter_p2p = p2p.call_as(&Addr::unchecked(COUNTER_TRADER));

    counter_p2p.suggest_counter_trade(trade_id, None)?;
    counter_p2p.add_asset(
        AddAssetAction::ToLastCounterTrade { trade_id },
        AssetInfo::Sg721Token(Sg721Token {
            address: nft.address()?.to_string(),
            token_id: COUNTER_ID.to_string(),
        }),
        &[],
    )?;

    counter_p2p.confirm_counter_trade(trade_id, None)?;

    let counter_id = 0;
    p2p.accept_trade(counter_id, trade_id, None)?;
    p2p.withdraw_successful_trade(trade_id, &coins(FEE_AMOUNT, FEE_DENOM))?;
    counter_p2p.withdraw_successful_trade(trade_id, &coins(FEE_AMOUNT, FEE_DENOM))?;

    let treasury_balance = chain.balance(treasury, Some(FEE_DENOM.to_string()))?;
    assert_eq!(treasury_balance, coins(2 * FEE_AMOUNT, FEE_DENOM));

    Ok(())
}

#[test]
fn with_funds() -> anyhow::Result<()> {
    let rt = Runtime::new()?;
    let mut chain = CloneTesting::new(&rt, STARGAZE_1)?;
    chain.set_sender(Addr::unchecked(OWNER));

    let treasury = chain.app.borrow().api().addr_make("treasury");

    let nft: Cw721<CloneTesting> = Cw721::new("geckies", chain.clone());
    nft.set_address(&Addr::unchecked(GECKIES_ADDRESS));
    let sns = Cw721::new("sns", chain.clone());
    sns.set_address(&Addr::unchecked(SNS_ADDRESS));

    let p2p = P2PTrading::new(chain.clone());
    p2p.upload()?;
    p2p.instantiate(
        &InstantiateMsg {
            name: "AtlasDaoTrading".to_string(),
            owner: None,
            accept_trade_fee: vec![
                coin(NICOCO_FEE_AMOUNT, "nicoco"),
                coin(FEE_AMOUNT, FEE_DENOM),
            ],
            fund_fee: FUND_FEE,
            treasury: treasury.to_string(),
        },
        None,
        None,
    )?;

    sns.execute(
        &ExecuteMsg::ApproveAll {
            operator: p2p.address()?.to_string(),
            expires: None,
        },
        None,
    )?;
    nft.call_as(&Addr::unchecked(COUNTER_TRADER)).execute(
        &ExecuteMsg::ApproveAll {
            operator: p2p.address()?.to_string(),
            expires: None,
        },
        None,
    )?;

    p2p.create_trade(None, None)?;
    p2p.add_asset(
        AddAssetAction::ToLastTrade {},
        AssetInfo::Sg721Token(Sg721Token {
            address: sns.address()?.to_string(),
            token_id: SNS.to_string(),
        }),
        &[],
    )?;

    p2p.get_chain().add_balance(
        &p2p.get_chain().sender(),
        vec![
            coin(FIRST_FUND_AMOUNT, "uarch"),
            coin(SECOND_FUND_AMOUNT, "uatlas"),
        ],
    )?;
    p2p.add_asset(
        AddAssetAction::ToLastTrade {},
        AssetInfo::Coin(coin(FIRST_FUND_AMOUNT, "uarch")),
        &coins(FIRST_FUND_AMOUNT, "uarch"),
    )?;
    p2p.add_asset(
        AddAssetAction::ToLastTrade {},
        AssetInfo::Coin(coin(SECOND_FUND_AMOUNT, "uatlas")),
        &coins(SECOND_FUND_AMOUNT, "uatlas"),
    )?;

    p2p.confirm_trade(None)?;
    let trade_id = 0;

    let counter_p2p = p2p.call_as(&Addr::unchecked(COUNTER_TRADER));

    counter_p2p.suggest_counter_trade(trade_id, None)?;
    counter_p2p.add_asset(
        AddAssetAction::ToLastCounterTrade { trade_id },
        AssetInfo::Sg721Token(Sg721Token {
            address: nft.address()?.to_string(),
            token_id: COUNTER_ID.to_string(),
        }),
        &[],
    )?;
    counter_p2p.get_chain().add_balance(
        &counter_p2p.get_chain().sender(),
        vec![
            coin(FIRST_FUND_AMOUNT, "ujuno"),
            coin(SECOND_FUND_AMOUNT, "uosmosis"),
        ],
    )?;
    counter_p2p.add_asset(
        AddAssetAction::ToLastCounterTrade { trade_id },
        AssetInfo::Coin(coin(FIRST_FUND_AMOUNT, "ujuno")),
        &coins(FIRST_FUND_AMOUNT, "ujuno"),
    )?;
    counter_p2p.add_asset(
        AddAssetAction::ToLastCounterTrade { trade_id },
        AssetInfo::Coin(coin(SECOND_FUND_AMOUNT, "uosmosis")),
        &coins(SECOND_FUND_AMOUNT, "uosmosis"),
    )?;

    counter_p2p.confirm_counter_trade(trade_id, None)?;

    let counter_id = 0;
    p2p.accept_trade(counter_id, trade_id, None)?;
    let response1 = p2p.withdraw_successful_trade(trade_id, &coins(FEE_AMOUNT, FEE_DENOM))?;
    let response2 =
        counter_p2p.withdraw_successful_trade(trade_id, &coins(FEE_AMOUNT, FEE_DENOM))?;

    let treasury_balance = chain.balance(treasury, None)?;

    let mut expected_coins = Coins::default();
    expected_coins.add(coin(2 * FEE_AMOUNT, FEE_DENOM))?;
    expected_coins.add(coin(
        (FUND_FEE * Uint128::from(FIRST_FUND_AMOUNT)).u128(),
        "uarch",
    ))?;
    expected_coins.add(coin(
        (FUND_FEE * Uint128::from(FIRST_FUND_AMOUNT)).u128(),
        "ujuno",
    ))?;
    expected_coins.add(coin(
        (FUND_FEE * Uint128::from(SECOND_FUND_AMOUNT)).u128(),
        "uatlas",
    ))?;
    expected_coins.add(coin(
        (FUND_FEE * Uint128::from(SECOND_FUND_AMOUNT)).u128(),
        "uosmosis",
    ))?;

    assert_eq!(treasury_balance, expected_coins.to_vec());

    Ok(())
}
