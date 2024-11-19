#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, Uint128};
    use std::vec;
    use utils::state::{AssetInfo, Sg721Token, NATIVE_DENOM};

    use crate::{
        common_setup::{
            helpers::plus_block_seconds,
            setup_accounts_and_block::{setup_accounts, setup_raffle_participants},
            setup_raffle::{proper_raffle_instantiate, DRAND_TIMEOUT},
        },
        raffle::setup::{
            execute_msg::{buy_tickets_template, create_raffle_setup},
            helpers::{finish_raffle_timeout, mint_one_token, raffle_info},
            test_msgs::{CreateRaffleParams, PurchaseTicketsParams},
        },
    };

    #[test]
    fn test_randomness_just_in_time() {
        let (mut app, contracts) = proper_raffle_instantiate();
        let (owner_addr, _, _) = setup_accounts(&mut app);
        let (one, two, three, _, _, _) = setup_raffle_participants(&mut app);
        let token = mint_one_token(&mut app, &contracts);
        // create raffle
        let params = CreateRaffleParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            owner_addr: owner_addr.clone(),
            creation_fee: vec![coin(4, NATIVE_DENOM)],
            ticket_price: Uint128::new(4),
            max_ticket_per_addr: None,
            raffle_start_timestamp: None,
            raffle_nfts: vec![AssetInfo::Sg721Token(Sg721Token {
                address: token.nft.to_string(),
                token_id: token.token_id.to_string(),
            })],
            duration: None,
            min_ticket_number: None,
            max_tickets: None,
            gating: vec![],
        };
        create_raffle_setup(params).unwrap();

        // Purchasing tickets for 1 person
        // ensure error if max tickets per address set is reached
        let params = PurchaseTicketsParams {
            app: &mut app,
            raffle_contract_addr: contracts.raffle.clone(),
            msg_senders: vec![one.clone()],
            raffle_id: 0,
            num_tickets: 10,
            funds_send: vec![coin(40, "ustars")],
        };
        // simulate the purchase of tickets
        buy_tickets_template(params).unwrap();

        // There is an error here
        finish_raffle_timeout(&mut app, &contracts, 0, 0).unwrap_err();
        plus_block_seconds(&mut app, 100 + DRAND_TIMEOUT);

        finish_raffle_timeout(&mut app, &contracts, 0, 0).unwrap();

        let res = raffle_info(&app, &contracts, 0).raffle_info.unwrap();
        assert_eq!(res.winners[0], one);
    }

    pub mod clone_testing {
        use std::io::Read;

        use cosmwasm_std::{coin, coins, Addr, Binary};
        use cw721_base::{interface::Cw721, ExecuteMsg};
        use cw_orch::daemon::RUNTIME;
        use cw_orch_clone_testing::{
            cw_multi_test::wasm_emulation::contract::{LocalWasmContract, WasmContract},
            CloneTesting,
        };
        use raffles::{
            msg::{DrandConfig, ExecuteMsgFns, MigrateMsg, QueryMsgFns},
            state::{RaffleOptionsMsg, RaffleState},
            Raffles,
        };
        use randomness_verifier::Verifier;
        use rustc_serialize::hex::FromHex;
        use utils::state::{AssetInfo, Sg721Token};

        use crate::{
            common_setup::{
                setup_minter::common::constants::CREATION_FEE_AMNT_STARS,
                setup_raffle::{DRAND_TIMEOUT, DRAND_URL, HEX_PUBKEY},
            },
            raffle::setup::helpers::{self, RANDOMNESS_1},
            trading::{COUNTER_ID, COUNTER_TRADER, GECKIES_ADDRESS, STARGAZE_1},
        };
        use cw_orch::prelude::*;

        pub struct RaffleUnlockEnv {
            chain: CloneTesting,
            raffles: Raffles<CloneTesting>,
            verifier: Verifier<CloneTesting>,
        }

        pub const ADMIN: &str = "stars1wk327tnqj03954zq2hzf36xzs656pmffzy0udsmjw2gjxrthh6qqfsvr4v";

        pub fn unlock_init() -> anyhow::Result<RaffleUnlockEnv> {
            let mut chain = CloneTesting::new(&RUNTIME, STARGAZE_1)?;
            let raffles = Raffles::new(chain.clone());
            //chain.set_sender(unchecked);

            let verifier = Verifier::new(chain.clone());
            verifier.upload()?;
            verifier.instantiate(&Empty {}, None, None)?;
            // Migrate raffles
            {
                let mut file =
                    std::fs::File::open(Raffles::<CloneTesting>::wasm(&STARGAZE_1.into()).path())?;

                let mut wasm = Vec::<u8>::new();
                file.read_to_end(&mut wasm)?;

                let new_code_id = chain
                    .app
                    .borrow_mut()
                    .store_wasm_code(WasmContract::Local(LocalWasmContract { code: wasm }));

                raffles.call_as(&Addr::unchecked(ADMIN)).migrate(
                    &MigrateMsg {
                        drand_config: DrandConfig {
                            random_pubkey: Binary::from(HEX_PUBKEY.from_hex()?),
                            drand_url: DRAND_URL.to_string(),
                            verify_signature_contract: verifier.address()?,
                            timeout: DRAND_TIMEOUT,
                        },
                    },
                    new_code_id,
                )?;
            }

            Ok(RaffleUnlockEnv {
                raffles,
                verifier,
                chain,
            })
        }
        #[test]
        // Clone Testing for locked raffles
        fn unlock_raffles() {
            let RaffleUnlockEnv {
                chain,
                raffles,
                verifier,
            } = unlock_init().unwrap();

            let raffle_ids = vec![134, 322, 323, 314];
            for id in raffle_ids {
                raffles.claim_raffle(id).unwrap_err();
                raffles
                    .update_randomness(id, helpers::rand(RANDOMNESS_1).unwrap())
                    .unwrap();

                raffles.claim_raffle(id).unwrap();

                let raffle_info = raffles.raffle_info(id).unwrap();
                assert_eq!(raffle_info.raffle_state, RaffleState::Claimed);
            }
        }

        #[test]
        // Clone Testing for locked raffles
        fn migrate_and_new_raffle() {
            let RaffleUnlockEnv {
                chain,
                raffles,
                verifier,
            } = unlock_init().unwrap();

            let nft: Cw721<CloneTesting> =
                Cw721::new("geckies", chain.clone()).call_as(&Addr::unchecked(COUNTER_TRADER));
            nft.set_address(&Addr::unchecked(GECKIES_ADDRESS));

            let config = raffles.config().unwrap();
            let id = config.last_raffle_id + 1;
            let ticket_price = coin(100_000, "ustars");

            chain
                .add_balance(&chain.sender_addr(), coins(1243872648274682746, "ustars"))
                .unwrap();

            nft.execute(
                &ExecuteMsg::ApproveAll {
                    operator: raffles.address().unwrap().to_string(),
                    expires: None,
                },
                None,
            )
            .unwrap();

            raffles
                .call_as(&Addr::unchecked(COUNTER_TRADER))
                .create_raffle(
                    vec![AssetInfo::Sg721Token(Sg721Token {
                        address: nft.address().unwrap().to_string(),
                        token_id: COUNTER_ID.to_string(),
                    })],
                    RaffleOptionsMsg {
                        raffle_start_timestamp: None,
                        raffle_duration: None,
                        comment: None,
                        max_ticket_number: None,
                        max_ticket_per_address: None,
                        raffle_preview: None,
                        one_winner_per_asset: true,
                        min_ticket_number: None,
                        whitelist: None,
                        gating_raffle: vec![],
                    },
                    AssetInfo::Coin(ticket_price.clone()),
                    None,
                    &[config.creation_coins[0].clone()],
                )
                .unwrap();

            raffles
                .buy_ticket(
                    id,
                    AssetInfo::Coin(ticket_price.clone()),
                    1,
                    None,
                    &[ticket_price],
                )
                .unwrap();

            chain.wait_seconds(100).unwrap();

            raffles.claim_raffle(id).unwrap_err();
            raffles
                .update_randomness(id, helpers::rand(RANDOMNESS_1).unwrap())
                .unwrap();
            chain.wait_seconds(DRAND_TIMEOUT).unwrap();

            raffles.claim_raffle(id).unwrap();

            let raffle_info = raffles.raffle_info(id).unwrap();
            assert_eq!(raffle_info.raffle_state, RaffleState::Claimed);
        }
    }
}
