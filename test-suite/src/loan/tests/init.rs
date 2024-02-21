#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, Decimal};

    use crate::{
        common_setup::{
            contract_boxes::custom_mock_app,
            setup_minter::common::constants::{
                LOAN_INTEREST_TAX, LOAN_NAME, MINT_PRICE, OWNER_ADDR,
            },
        },
        loan::setup::{execute_msg::instantate_loan_contract, test_msgs::InstantiateParams},
    };

    #[test]
    fn good_i() {
        let mut app = custom_mock_app();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: LOAN_INTEREST_TAX,
            name: LOAN_NAME.into(),
        };
        let i = instantate_loan_contract(params);
        assert!(i.is_ok());
    }

    #[test]
    fn bad_fee_rate() {
        let mut app = custom_mock_app();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: LOAN_INTEREST_TAX + Decimal::percent(60),
            name: LOAN_NAME.into(),
        };
        let res = instantate_loan_contract(params).unwrap_err();
        assert_eq!(
            res.root_cause().to_string(),
            "The fee_rate you provided is not greater than 0, or less than 1"
        );
    }

    #[test]
    fn bad_i_name() {
        let mut app = custom_mock_app();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: LOAN_INTEREST_TAX,
            name: "80808080808080808080808080808080808080808080808080808080808080808080808080808080808088080808080808080808080808080808080808080808080808080808080808080808080808080808080808".to_string(),
        };
        let res1 = instantate_loan_contract(params).unwrap_err();
        let params = InstantiateParams {
            app: &mut app,
            funds_amount: MINT_PRICE,
            admin_account: Addr::unchecked(OWNER_ADDR),
            fee_rate: LOAN_INTEREST_TAX,
            name: "80".to_string(),
        };
        let res2 = instantate_loan_contract(params).unwrap_err();
        assert_eq!(res1.root_cause().to_string(), "Invalid Name");
        assert_eq!(res2.root_cause().to_string(), "Invalid Name");
    }
}
