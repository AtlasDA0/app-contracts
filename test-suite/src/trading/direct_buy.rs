use cosmwasm_std::{coin, coins, Coins, Uint128};
use cw721_base::interface::{Cw721, ExecuteMsg};
use cw_orch::{daemon::RUNTIME, environment::Environment, prelude::*};
use cw_orch_clone_testing::CloneTesting;
use p2p_trading::P2PTrading;
use p2p_trading_export::msg::{AddAssetAction, ExecuteMsgFns, InstantiateMsg};
use sg721_base::msg::CollectionInfoResponse;
use utils::state::{AssetInfo, Sg721Token};

use crate::trading::{COUNTER_TRADER, FIRST_FUND_AMOUNT, SECOND_FUND_AMOUNT, SNS};

use super::{
    COUNTER_ID, FEE_AMOUNT, FEE_DENOM, FUND_FEE, GECKIES_ADDRESS, NICOCO_FEE_AMOUNT, OWNER,
    SNS_ADDRESS, STARGAZE_1,
};

pub struct TradingTestEnv {
    chain: CloneTesting,
    treasury: Addr,
    nft: Cw721<CloneTesting>,
    sns: Cw721<CloneTesting>,
    p2p: P2PTrading<CloneTesting>,
}

fn direct_buy_init() -> anyhow::Result<TradingTestEnv> {
    let mut chain_info = STARGAZE_1;
    // chain_info.grpc_urls = &["http://stargaze-grpc.polkachu.com:13790"];
    let mut chain = CloneTesting::new(&RUNTIME, chain_info)?;
    chain.set_sender(Addr::unchecked(COUNTER_TRADER));

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
    p2p.create_trade(None, None)?;
    nft.execute(
        &ExecuteMsg::ApproveAll {
            operator: p2p.address()?.to_string(),
            expires: None,
        },
        None,
    )?;
    p2p.add_asset(
        AddAssetAction::ToLastTrade {},
        AssetInfo::Sg721Token(Sg721Token {
            address: nft.address()?.to_string(),
            token_id: COUNTER_ID.to_string(),
        }),
        &[],
    )?;

    Ok(TradingTestEnv {
        nft,
        sns,
        p2p,
        chain,
        treasury,
    })
}

#[test]
fn direct_buy_works() -> anyhow::Result<()> {
    let TradingTestEnv {
        nft: _,
        sns: _,
        p2p,
        chain,
        treasury,
    } = direct_buy_init()?;

    p2p.add_tokens_wanted(coins(FIRST_FUND_AMOUNT, "ujuno"), None)?;

    p2p.confirm_trade(None)?;
    let trade_id = 0;

    let counter_p2p = p2p.call_as(&Addr::unchecked(OWNER));

    counter_p2p.environment().add_balance(
        &counter_p2p.environment().sender_addr(),
        vec![
            coin(FIRST_FUND_AMOUNT, "ujuno"),
            coin(SECOND_FUND_AMOUNT, "uosmosis"),
        ],
    )?;
    counter_p2p.direct_buy(0, None, &coins(FIRST_FUND_AMOUNT, "ujuno"))?;

    p2p.withdraw_successful_trade(trade_id, &coins(FEE_AMOUNT, FEE_DENOM))?;

    let treasury_balance = chain.balance(treasury, None)?;

    let mut expected_coins = Coins::default();
    expected_coins.add(coin(FEE_AMOUNT, FEE_DENOM))?;
    expected_coins.add(coin(
        (FUND_FEE * Uint128::from(FIRST_FUND_AMOUNT)).u128(),
        "ujuno",
    ))?;

    assert_eq!(treasury_balance, expected_coins.to_vec());

    Ok(())
}

#[test]
fn direct_buy_insufficient_funds() -> anyhow::Result<()> {
    let TradingTestEnv {
        nft: _,
        sns: _,
        p2p,
        chain: _,
        treasury: _,
    } = direct_buy_init()?;

    p2p.add_tokens_wanted(coins(FIRST_FUND_AMOUNT, "ujuno"), None)?;

    p2p.confirm_trade(None)?;
    let trade_id = 0;

    let counter_p2p = p2p.call_as(&Addr::unchecked(OWNER));

    counter_p2p.environment().add_balance(
        &counter_p2p.environment().sender_addr(),
        vec![
            coin(FIRST_FUND_AMOUNT, "ujuno"),
            coin(SECOND_FUND_AMOUNT, "uosmosis"),
        ],
    )?;
    counter_p2p
        .direct_buy(0, None, &coins(FIRST_FUND_AMOUNT - 1, "ujuno"))
        .unwrap_err();

    Ok(())
}

#[test]
fn direct_buy_cant_counter_after() -> anyhow::Result<()> {
    let TradingTestEnv {
        nft: _,
        sns: _,
        p2p,
        chain: _,
        treasury: _,
    } = direct_buy_init()?;

    p2p.add_tokens_wanted(coins(FIRST_FUND_AMOUNT, "ujuno"), None)?;

    p2p.confirm_trade(None)?;
    let trade_id = 0;

    let counter_p2p = p2p.call_as(&Addr::unchecked(OWNER));

    counter_p2p.environment().add_balance(
        &counter_p2p.environment().sender_addr(),
        vec![
            coin(FIRST_FUND_AMOUNT, "ujuno"),
            coin(SECOND_FUND_AMOUNT, "uosmosis"),
        ],
    )?;
    counter_p2p.direct_buy(0, None, &coins(FIRST_FUND_AMOUNT, "ujuno"))?;

    counter_p2p
        .suggest_counter_trade(trade_id, None)
        .unwrap_err();

    Ok(())
}

#[test]
fn direct_buy_respects_royalties() -> anyhow::Result<()> {
    let TradingTestEnv {
        nft,
        sns: _,
        p2p,
        chain,
        treasury: _,
    } = direct_buy_init()?;

    let nft_collection_info: CollectionInfoResponse = chain.query(
        &sg721_base::msg::QueryMsg::CollectionInfo {},
        &nft.address()?,
    )?;
    let royalty_info = nft_collection_info.royalty_info.unwrap();

    let royalty_balance_before = chain.balance(
        royalty_info.payment_address.clone(),
        Some("ujuno".to_string()),
    )?;

    p2p.add_tokens_wanted(coins(FIRST_FUND_AMOUNT, "ujuno"), None)?;

    p2p.confirm_trade(None)?;
    let trade_id = 0;

    let counter_p2p = p2p.call_as(&Addr::unchecked(OWNER));

    counter_p2p.environment().add_balance(
        &counter_p2p.environment().sender_addr(),
        vec![
            coin(FIRST_FUND_AMOUNT, "ujuno"),
            coin(SECOND_FUND_AMOUNT, "uosmosis"),
        ],
    )?;
    counter_p2p.direct_buy(0, None, &coins(FIRST_FUND_AMOUNT, "ujuno"))?;

    p2p.withdraw_successful_trade(trade_id, &coins(FEE_AMOUNT, FEE_DENOM))?;

    let royalty_balance_after =
        chain.balance(royalty_info.payment_address, Some("ujuno".to_string()))?;

    assert_eq!(
        royalty_balance_after[0].amount,
        royalty_balance_before[0].amount
            + Uint128::from(FIRST_FUND_AMOUNT).mul_floor(royalty_info.share),
    );

    Ok(())
}

#[test]
fn direct_buy_works_without_accept_fee() -> anyhow::Result<()> {
    let mut chain_info = STARGAZE_1;
    let mut chain = CloneTesting::new(&RUNTIME, chain_info)?;
    chain.set_sender(Addr::unchecked(COUNTER_TRADER));

    let treasury = chain.app.borrow().api().addr_make("treasury");

    let nft: Cw721<CloneTesting> = Cw721::new("geckies", chain.clone());
    nft.set_address(&Addr::unchecked(GECKIES_ADDRESS));

    let p2p = P2PTrading::new(chain.clone());
    p2p.upload()?;

    // Instantiate with empty accept_trade_fee
    p2p.instantiate(
        &InstantiateMsg {
            name: "AtlasDaoTrading".to_string(),
            owner: None,
            accept_trade_fee: vec![], // Empty vector for no accept fee
            fund_fee: FUND_FEE,
            treasury: treasury.to_string(),
        },
        None,
        None,
    )?;

    // Create and set up the trade
    p2p.create_trade(None, None)?;
    nft.execute(
        &ExecuteMsg::ApproveAll {
            operator: p2p.address()?.to_string(),
            expires: None,
        },
        None,
    )?;
    p2p.add_asset(
        AddAssetAction::ToLastTrade {},
        AssetInfo::Sg721Token(Sg721Token {
            address: nft.address()?.to_string(),
            token_id: COUNTER_ID.to_string(),
        }),
        &[],
    )?;

    // Add tokens wanted and confirm trade
    p2p.add_tokens_wanted(coins(FIRST_FUND_AMOUNT, "ujuno"), None)?;
    p2p.confirm_trade(None)?;
    let trade_id = 0;

    // Execute direct buy as counter trader
    let counter_p2p = p2p.call_as(&Addr::unchecked(OWNER));
    counter_p2p.environment().add_balance(
        &counter_p2p.environment().sender_addr(),
        vec![coin(FIRST_FUND_AMOUNT, "ujuno")],
    )?;
    counter_p2p.direct_buy(0, None, &coins(FIRST_FUND_AMOUNT, "ujuno"))?;

    // Withdraw successful trade without any accept fee
    p2p.withdraw_successful_trade(trade_id, &[])?;

    // Check treasury balance - should only have fund fee
    let treasury_balance = chain.balance(treasury, None)?;
    let expected_coins = vec![coin(
        (FUND_FEE * Uint128::from(FIRST_FUND_AMOUNT)).u128(),
        "ujuno",
    )];

    assert_eq!(treasury_balance, expected_coins);

    Ok(())
}
