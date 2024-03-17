use std::ops::{Deref, DerefMut};

use cosmwasm_std::{
    testing::{mock_env, MockApi, MockStorage},
    Decimal, Empty, Querier, QuerierResult, Validator,
};
use cw_multi_test::{App, BankKeeper, BasicAppBuilder, WasmKeeper};
use sg_multi_test::StargazeModule;
use sg_std::StargazeMsgWrapper;

pub type StargazeBasicApp =
    App<BankKeeper, MockApi, MockStorage, StargazeModule, WasmKeeper<StargazeMsgWrapper, Empty>>;

pub struct StargazeApp(StargazeBasicApp);

impl Deref for StargazeApp {
    type Target = StargazeBasicApp;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StargazeApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Querier for StargazeApp {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        self.0.raw_query(bin_request)
    }
}

impl StargazeApp {
    pub fn new() -> Self {
        Self(
            BasicAppBuilder::<StargazeMsgWrapper, Empty>::new_custom()
                .with_custom(StargazeModule {})
                .build(|router, api, storage| {
                    router
                        .staking
                        .add_validator(
                            api,
                            storage,
                            &mock_env().block,
                            Validator {
                                address: "validator".to_string(),
                                commission: Decimal::percent(10),
                                max_commission: Decimal::percent(20),
                                max_change_rate: Decimal::percent(20),
                            },
                        )
                        .unwrap();
                }),
        )
    }
}

impl Default for StargazeApp {
    fn default() -> Self {
        Self::new()
    }
}
