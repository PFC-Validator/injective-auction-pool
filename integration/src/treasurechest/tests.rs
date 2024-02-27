use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{assert_approx_eq, Addr, Coin, Decimal, Uint128};
use cw_multi_test::{error::AnyResult, App, AppBuilder, Executor};

use crate::{get_attribute, get_events, treasurechest::contract::TreasureChestContract};

// ADMIN of the contract
pub const ADMIN: &str = "admin1";
pub const USER1: &str = "user1";
pub const USER2: &str = "user2";
pub const USER_BAD: &str = "user3_bad";
pub const NATIVE_DENOM: &str = "denom";
pub const TICKET_DENOM: &str = "ticket_denom";
pub const DENOM1: &str = "denom1";
pub const DENOM2: &str = "denom2";
const DENOM1_AMT: Uint128 = Uint128::new(100_000u128);
const DENOM2_AMT: Uint128 = Uint128::new(37_000_000u128);
const TICKET_AMT: Uint128 = Uint128::new(73u128);
const TICKET_USER1: u128 = 70;
const TICKET_USER2: u128 = 3;

pub fn mock_app() -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(USER1),
                vec![
                    Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    },
                    Coin {
                        denom: TICKET_DENOM.to_string(),
                        amount: Uint128::new(TICKET_USER1),
                    },
                ],
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(USER2),
                vec![
                    Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    },
                    Coin {
                        denom: TICKET_DENOM.to_string(),
                        amount: Uint128::new(TICKET_USER2),
                    },
                ],
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(ADMIN),
                vec![
                    Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(100),
                    },
                    Coin {
                        denom: DENOM1.to_string(),
                        amount: DENOM1_AMT,
                    },
                    Coin {
                        denom: DENOM2.to_string(),
                        amount: DENOM2_AMT,
                    },
                ],
            )
            .unwrap()
    })
}

pub fn instantiate(app: &mut App, burn: bool, coins: &[Coin]) -> AnyResult<TreasureChestContract> {
    //let admin = app.api().addr_make(ADMIN);
    let code_id = app.store_code(TreasureChestContract::template());
    let msg = TreasureChestContract::simple_instantiate(ADMIN, TICKET_DENOM, burn);
    let contract = app.instantiate_contract(
        code_id,
        Addr::unchecked(ADMIN),
        &msg,
        coins,
        "treasure-chest",
        Some(Addr::unchecked(ADMIN).to_string()),
    )?;
    Ok(TreasureChestContract(contract))
}

#[test]
pub fn test_instantiate() -> AnyResult<()> {
    let mut app = mock_app();
    let chest = instantiate(
        &mut app,
        true,
        &[
            Coin {
                denom: DENOM1.to_string(),
                amount: DENOM1_AMT,
            },
            Coin {
                denom: DENOM2.to_string(),
                amount: DENOM2_AMT,
            },
        ],
    )?;

    let config = chest.config(&app)?;

    assert_eq!(config.owner, ADMIN);
    assert_eq!(config.token_factory_type.to_string(), "CosmWasm");
    assert!(config.burn_it);
    let state = chest.state(&app)?;
    assert_eq!(state.denom, TICKET_DENOM);
    assert_eq!(state.outstanding, TICKET_AMT);
    assert_eq!(state.holding, Uint128::zero());
    let rewards_single: HashMap<String, Decimal> =
        HashMap::from_iter(state.rewards_per_one_token.iter().map(|x| (x.denom.clone(), x.amount)));

    let denom1_amt = rewards_single.get(DENOM1).unwrap().to_uint_floor() * state.outstanding;

    assert_approx_eq!(DENOM1_AMT, denom1_amt, "0.001");
    let denom2_amt = rewards_single.get(DENOM2).unwrap().to_uint_floor() * state.outstanding;

    assert_approx_eq!(DENOM2_AMT, denom2_amt, "0.001");

    Ok(())
}

#[test]
pub fn test_withdraw() -> AnyResult<()> {
    let mut app = mock_app();
    let chest = instantiate(
        &mut app,
        false,
        &[
            Coin {
                denom: DENOM1.to_string(),
                amount: DENOM1_AMT,
            },
            Coin {
                denom: DENOM2.to_string(),
                amount: DENOM2_AMT,
            },
        ],
    )?;
    let mut remain_denom1 = DENOM1_AMT;
    let mut remain_denom2 = DENOM2_AMT;

    let err = app
        .execute(Addr::unchecked(USER1), chest.withdraw(vec![])?)
        .unwrap_err()
        .root_cause()
        .to_string();

    assert_eq!("need to send ticket_denom tokens", err);
    let err = app
        .execute(
            Addr::unchecked(USER1),
            chest.withdraw(vec![Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::one(),
            }])?,
        )
        .unwrap_err()
        .root_cause()
        .to_string();

    assert_eq!("only ticket_denom tokens", err);
    let err = app
        .execute(
            Addr::unchecked(USER1),
            chest.withdraw(vec![
                Coin {
                    denom: TICKET_DENOM.to_string(),
                    amount: Uint128::one(),
                },
                Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::one(),
                },
            ])?,
        )
        .unwrap_err()
        .root_cause()
        .to_string();

    assert_eq!("only ticket_denom tokens", err);
    let res = app.execute(
        Addr::unchecked(USER1),
        chest.withdraw(vec![Coin {
            denom: TICKET_DENOM.to_string(),
            amount: Uint128::one(),
        }])?,
    )?;
    app.update_block(|f| f.height += 1);
    let state = chest.state(&app)?;
    let rewards_single: HashMap<String, Decimal> =
        HashMap::from_iter(state.rewards_per_one_token.iter().map(|x| (x.denom.clone(), x.amount)));
    let d1 = rewards_single.get(DENOM1).unwrap();
    let d2 = rewards_single.get(DENOM2).unwrap();
    assert_eq!(state.outstanding, Uint128::new(72));
    for e in get_events("transfer", &res.events) {
        assert_eq!(get_attribute("recipient", &e.attributes).unwrap(), USER1);
        if let Some(amount) = get_attribute("amount", &e.attributes) {
            let coins = amount.split(',').map(|x| x.to_string()).collect::<Vec<_>>();
            assert_eq!(coins.len(), 2);
            remain_denom1 -= d1.to_uint_floor();
            remain_denom2 -= d2.to_uint_floor();

            assert_eq!(coins.first().unwrap(), &format!("{}denom1", d1.floor()));
            assert_eq!(coins.get(1).unwrap(), &format!("{}denom2", d2.floor()));
        } else {
            eprintln!("{:?}", e.attributes);
            panic!("no amount?")
        }
    }
    app.update_block(|f| f.height += 1);
    let res_10 = app.execute(
        Addr::unchecked(USER1),
        chest.withdraw(vec![Coin {
            denom: TICKET_DENOM.to_string(),
            amount: Uint128::new(10),
        }])?,
    )?;

    let state_2 = chest.state(&app)?;
    let rewards_single_2: HashMap<String, Decimal> = HashMap::from_iter(
        state_2.rewards_per_one_token.iter().map(|x| (x.denom.clone(), x.amount)),
    );
    let d1_2 = rewards_single_2.get(DENOM1).unwrap();
    let d2_2 = rewards_single_2.get(DENOM2).unwrap();
    assert_eq!(d1, d1_2);
    assert_eq!(d2, d2_2);
    assert_eq!(state_2.outstanding, Uint128::new(62));
    for e in get_events("transfer", &res_10.events) {
        assert_eq!(get_attribute("recipient", &e.attributes).unwrap(), USER1);
        if let Some(amount) = get_attribute("amount", &e.attributes) {
            let coins = amount.split(',').map(|x| x.to_string()).collect::<Vec<_>>();
            assert_eq!(coins.len(), 2);
            let amt1 = (d1_2 * Decimal::from_str("10")?).floor();
            assert_eq!(coins.first().unwrap(), &format!("{}denom1", amt1));
            let amt2 = (d2_2 * Decimal::from_str("10")?).floor();
            assert_eq!(coins.get(1).unwrap(), &format!("{}denom2", amt2));
            remain_denom1 -= amt1.to_uint_floor();
            remain_denom2 -= amt2.to_uint_floor();
        } else {
            eprintln!("{:?}", e.attributes);
            panic!("no amount?")
        }
    }
    app.update_block(|f| f.height += 1);
    let res_user1 = app.execute(
        Addr::unchecked(USER1),
        chest.withdraw(vec![Coin {
            denom: TICKET_DENOM.to_string(),
            amount: Uint128::new(59),
        }])?,
    )?;
    for e in get_events("transfer", &res_user1.events) {
        assert_eq!(get_attribute("recipient", &e.attributes).unwrap(), USER1);
        if let Some(amount) = get_attribute("amount", &e.attributes) {
            let coins = amount.split(',').map(|x| x.to_string()).collect::<Vec<_>>();
            assert_eq!(coins.len(), 2);
            let amt1 = (d1_2 * Decimal::from_str("59")?).floor();
            assert_eq!(coins.first().unwrap(), &format!("{}denom1", amt1));
            let amt2 = (d2_2 * Decimal::from_str("59")?).floor();
            assert_eq!(coins.get(1).unwrap(), &format!("{}denom2", amt2));
            remain_denom1 -= amt1.to_uint_floor();
            remain_denom2 -= amt2.to_uint_floor();
        } else {
            eprintln!("{:?}", e.attributes);
            panic!("no amount?")
        }
    }
    //   eprintln!("Remaining (calc) {}{} {}{}", remain_denom1, DENOM1, remain_denom2, DENOM2);

    let bal = app
        .wrap()
        .query_all_balances(chest.0.clone())?
        .into_iter()
        .filter(|c| c.denom != TICKET_DENOM)
        .map(|c| (c.denom, c.amount))
        .collect::<HashMap<String, Uint128>>();
    assert_eq!(remain_denom1, bal.get(DENOM1).unwrap());
    assert_eq!(remain_denom2, bal.get(DENOM2).unwrap());
    app.update_block(|f| f.height += 1);
    let res_user2 = app.execute(
        Addr::unchecked(USER2),
        chest.withdraw(vec![Coin {
            denom: TICKET_DENOM.to_string(),
            amount: Uint128::new(TICKET_USER2),
        }])?,
    )?;
    for e in get_events("transfer", &res_user2.events) {
        assert_eq!(get_attribute("recipient", &e.attributes).unwrap(), USER2);
        if let Some(amount) = get_attribute("amount", &e.attributes) {
            let coins = amount.split(',').map(|x| x.to_string()).collect::<Vec<_>>();
            assert_eq!(coins.len(), 2);
            let amt1 = (d1_2 * Decimal::from_str(&TICKET_USER2.to_string())?).floor();
            assert_eq!(coins.first().unwrap(), &format!("{}denom1", amt1));
            let amt2 = (d2_2 * Decimal::from_str(&TICKET_USER2.to_string())?).floor();
            assert_eq!(coins.get(1).unwrap(), &format!("{}denom2", amt2));
            remain_denom1 -= amt1.to_uint_floor();
            remain_denom2 -= amt2.to_uint_floor();
        } else {
            eprintln!("{:?}", e.attributes);
            panic!("no amount?")
        }
    }
    //  eprintln!("Remaining (calc) {}{} {}{}", remain_denom1, DENOM1, remain_denom2, DENOM2);

    let bal = app
        .wrap()
        .query_all_balances(chest.0.clone())?
        .into_iter()
        .filter(|c| c.denom != TICKET_DENOM)
        .map(|c| (c.denom, c.amount))
        .collect::<HashMap<String, Uint128>>();
    assert_eq!(remain_denom1, bal.get(DENOM1).unwrap());
    assert_eq!(remain_denom2, bal.get(DENOM2).unwrap());

    let state = chest.state(&app)?;

    assert_eq!(state.denom, TICKET_DENOM);
    assert_eq!(state.holding, TICKET_AMT);
    assert_eq!(state.outstanding, Uint128::zero());

    app.update_block(|f| f.height += 1);
    let res_admin = app.execute(Addr::unchecked(ADMIN), chest.return_dust()?)?;
    for e in get_events("transfer", &res_admin.events) {
        if let Some(amount) = get_attribute("amount", &e.attributes) {
            let coins = amount.split(',').map(|x| x.to_string()).collect::<Vec<_>>();
            assert_eq!(coins.len(), 2);
            assert_eq!(coins.first().unwrap(), "3denom1");
            assert_eq!(coins.get(1).unwrap(), "2denom2");
        }
    }
    assert_eq!(remain_denom1, Uint128::new(3));
    assert_eq!(remain_denom2, Uint128::new(2));
    let state = chest.state(&app)?;

    assert!(state.rewards_per_one_token.is_empty());

    Ok(())
}
