mod clone_testing;
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
