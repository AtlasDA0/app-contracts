use cw_orch::daemon::{ChainInfo, ChainKind, NetworkInfo};

pub mod raffles;

pub const STARGAZE_NETWORK: NetworkInfo = NetworkInfo {
    id: "stargaze",
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
