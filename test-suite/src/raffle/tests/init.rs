#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Addr, Decimal};
    use utils::state::NATIVE_DENOM;

    use crate::common_setup::{
        contract_boxes::custom_mock_app,
        setup_minter::common::constants::{OWNER_ADDR, RAFFLE_NAME},
        setup_raffle::{default_drand_config, proper_raffle_instantiate, raffle_template_code_ids},
    };
    use cw_multi_test::Executor;
    use raffles::{error::ContractError, msg::InstantiateMsg};

    #[test]
    fn test_i() {
        proper_raffle_instantiate();
    }

    #[test]
    fn test_i_error_high_fee_rate() {
        let mut app = custom_mock_app();

        let code_ids = raffle_template_code_ids(&mut app);
        let res = app
            .instantiate_contract(
                code_ids.raffle_code_id,
                Addr::unchecked(OWNER_ADDR),
                &InstantiateMsg {
                    name: RAFFLE_NAME.to_string(),
                    owner: Some(OWNER_ADDR.to_string()),
                    fee_addr: Some("atlas-treasury-placeholder".to_owned()),
                    minimum_raffle_duration: None,
                    max_ticket_number: None,
                    raffle_fee: Decimal::percent(200),
                    creation_coins: vec![
                        coin(4, NATIVE_DENOM.to_string()),
                        coin(20, "ustars".to_string()),
                    ]
                    .into(),
                    fee_discounts: vec![],
                    drand_config: default_drand_config(&Addr::unchecked("any")),
                },
                &[],
                "raffle",
                Some(Addr::unchecked(OWNER_ADDR).to_string()),
            )
            .unwrap_err();

        assert_eq!(
            res.root_cause().to_string(),
            ContractError::InvalidFeeRate {}.to_string()
        )
    }
}
