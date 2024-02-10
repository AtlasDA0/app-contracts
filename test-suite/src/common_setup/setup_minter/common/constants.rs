use cosmwasm_std::Decimal;

// pub const LISTING_FEE: u128 = 0;
pub const AIRDROP_MINT_PRICE: u128 = 0;
pub const AIRDROP_MINT_FEE_FAIR_BURN: u64 = 10_000; // 100%
pub const CREATION_FEE: u128 = 5_000_000_000;
pub const CREATION_FEE_AMNT: u128 = 50;
pub const MAX_PER_ADDRESS_LIMIT: u32 = 50;
pub const MAX_TOKEN_LIMIT: u32 = 10000;
pub const MIN_COLLATERAL_LISTING: u128 = 10;
pub const LOAN_INTEREST_TAX: Decimal = Decimal::percent(50); // 50%
pub const RAFFLE_TAX: Decimal = Decimal::percent(50);
pub const MIN_MINT_PRICE: u128 = 50_000_000;
pub const MIN_MINT_PRICE_OPEN_EDITION: u128 = 100_000_000;
pub const MINT_FEE_FAIR_BURN: u64 = 1_000; // 10%
pub const MINT_PRICE: u128 = 100_000_000;
pub const SHUFFLE_FEE: u128 = 500_000_000;
// const NOIS_AMOUNT: u128 = 50;

pub const OWNER_ADDR: &str = "fee";
pub const OFFERER_ADDR: &str = "offerer";
pub const DEPOSITOR_ADDR: &str = "depositor";
pub const BORROWER_ADDR: &str = "borrower";
pub const TREASURY_ADDR: &str = "collector";
pub const VENDING_MINTER: &str = "contract2";
pub const SG721_CONTRACT: &str = "contract3";
pub const NOIS_PROXY_ADDR: &str = "nois";
pub const RAFFLE_NAME: &str = "raffle contract name";
pub const LOAN_NAME: &str = "loan-with-insights";

pub const DEV_ADDRESS: &str = "stars1abcd4kdla12mh86psg4y4h6hh05g2hmqoap350";
pub const FOUNDATION: &str = "stars1xqz6xujjyz0r9uzn7srasle5uynmpa0zkjr5l8";

const NOIS_AMOUNT: u128 = 50;
const NAME: &str = "raffle param name";

