mod clone_testing;
mod direct_buy;
use cosmwasm_std::Decimal;
use cw_orch::environment::{ChainInfo, ChainKind, NetworkInfo};

pub const STARGAZE_NETWORK: NetworkInfo = NetworkInfo {
    chain_name: "stargaze",
    pub_address_prefix: "stars",
    coin_type: 118u32,
};

/// https://github.com/cosmos/chain-registry/blob/master/testnets/stargazetestnet/chain.json
pub const ELGAFAR_1: ChainInfo = ChainInfo {
    kind: ChainKind::Testnet,
    chain_id: "elgafar-1",
    gas_denom: "ustars",
    gas_price: 0.04,
    grpc_urls: &["http://grpc-1.elgafar-1.stargaze-apis.com:26660"],
    network_info: STARGAZE_NETWORK,
    lcd_url: None,
    fcd_url: None,
};

/// https://github.com/cosmos/chain-registry/blob/master/testnets/stargazetestnet/chain.json
pub const STARGAZE_1: ChainInfo = ChainInfo {
    kind: ChainKind::Mainnet,
    chain_id: "stargaze-1",
    gas_denom: "ustars",
    gas_price: 1.1,
    grpc_urls: &["http://stargaze-grpc.polkachu.com:13790"],
    network_info: STARGAZE_NETWORK,
    lcd_url: None,
    fcd_url: None,
};

pub const SNS_ADDRESS: &str = "stars1fx74nkqkw2748av8j7ew7r3xt9cgjqduwn8m0ur5lhe49uhlsasszc5fhr";
pub const GECKIES_ADDRESS: &str =
    "stars166kqwcu8789xh7nk07fcrdzek54205u8gzas684lnas2kzalksqsg5xhqf";
pub const GECKIES_ID: &str = "790";

pub const SNS: &str = "jacobremy";
pub const OWNER: &str = "stars1f4fqgj2htmpff6qe5nnhgl42pevekl34ykdah9";

pub const FIRST_FUND_AMOUNT: u128 = 485;
pub const SECOND_FUND_AMOUNT: u128 = 456;

pub const COUNTER_TRADER: &str = "stars1s46jmv3c05usk6yk50tyy8axc4t9rglrvdky2u";
pub const COUNTER_ID: &str = "2887";

pub const NICOCO_FEE_AMOUNT: u128 = 498579754654;
pub const FEE_AMOUNT: u128 = 4514987;
pub const FEE_DENOM: &str = "ustars";

pub const FUND_FEE: Decimal = Decimal::percent(3);
