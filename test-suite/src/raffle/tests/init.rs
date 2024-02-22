#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Decimal};
    use utils::state::{NATIVE_DENOM, NOIS_AMOUNT};

    use raffles::error::ContractError;

    use crate::{
        common_setup::{
            contract_boxes::custom_mock_app,
            helpers::assert_error,
            setup_minter::common::constants::{
                MINT_PRICE, NOIS_PROXY_ADDR, OWNER_ADDR, RAFFLE_NAME, RAFFLE_TAX,
            },
        },
        raffle::setup::{
            execute_msg::instantate_raffle_contract, test_msgs::InstantiateRaffleParams,
        },
    };

    #[test]
    fn test_i() {
        let mut app = custom_mock_app();
        let params = InstantiateRaffleParams {
            app: &mut app,
            admin_account: Addr::unchecked(OWNER_ADDR),
            funds_amount: MINT_PRICE,
            fee_rate: RAFFLE_TAX,
            name: RAFFLE_NAME.into(),
            nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
            nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
        };
        let instantiate = instantate_raffle_contract(params);
        assert!(
            instantiate.is_ok(),
            "There is an issue instantiating a raffle"
        );
    }

    #[test]
    fn test_i_error_high_fee_rate() {
        let mut app = custom_mock_app();
        let params = InstantiateRaffleParams {
            app: &mut app,
            admin_account: Addr::unchecked(OWNER_ADDR),
            funds_amount: MINT_PRICE,
            fee_rate: Decimal::percent(200),
            name: RAFFLE_NAME.into(),
            nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
            nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
        };
        let res = instantate_raffle_contract(params).unwrap_err();
        assert_eq!(
            res.root_cause().to_string(),
            "The fee_rate you provided is not greater than 0, or less than 1"
        )
    }

    #[test]
    fn test_i_bad_nois_proxy_addr() {
        let mut app = custom_mock_app();
        let params = InstantiateRaffleParams {
            app: &mut app,
            admin_account: Addr::unchecked(OWNER_ADDR),
            funds_amount: MINT_PRICE,
            fee_rate: Decimal::percent(200),
            name: RAFFLE_NAME.into(),
            nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
            nois_proxy_addr: "".to_string(),
        };
        let res = instantate_raffle_contract(params).unwrap_err();
        assert_error(Err(res), ContractError::InvalidProxyAddress {}.to_string())
    }

    #[test]
    fn test_i_name() {
        let mut app = custom_mock_app();
        let params = InstantiateRaffleParams {
            app: &mut app,
            admin_account: Addr::unchecked(OWNER_ADDR),
            funds_amount: MINT_PRICE,
            fee_rate: RAFFLE_TAX,
            name: "80808080808080808080808080808080808080808080808080808080808080808080808080808080808088080808080808080808080808080808080808080808080808080808080808080808080808080808080808".to_string(),
            nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
            nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
        };
        let res1 = instantate_raffle_contract(params).unwrap_err();
        let params = InstantiateRaffleParams {
            app: &mut app,
            admin_account: Addr::unchecked(OWNER_ADDR),
            funds_amount: MINT_PRICE,
            fee_rate: RAFFLE_TAX,
            name: "80".to_string(),
            nois_proxy_coin: coin(NOIS_AMOUNT, NATIVE_DENOM),
            nois_proxy_addr: NOIS_PROXY_ADDR.to_string(),
        };
        let res2 = instantate_raffle_contract(params).unwrap_err();
        assert_eq!(res1.root_cause().to_string(), "Invalid Name");
        assert_eq!(res2.root_cause().to_string(), "Invalid Name");
    }
}
