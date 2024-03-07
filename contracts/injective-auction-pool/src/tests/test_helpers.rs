use std::marker::PhantomData;

use cosmwasm_std::testing::{
    mock_env, mock_info, BankQuerier, MockApi, MockStorage, MOCK_CONTRACT_ADDR,
};
use cosmwasm_std::{
    attr, coins, from_json, to_json_binary, BankMsg, Binary, ContractResult as CwContractResult,
    CosmosMsg, Decimal, Empty, Env, MemoryStorage, MessageInfo, OwnedDeps, Querier, QuerierResult,
    QueryRequest, Uint128, WasmMsg,
};
use injective_auction::auction::{Coin, QueryCurrentAuctionBasketResponse};
use injective_auction::auction_pool::{ExecuteMsg, InstantiateMsg};
use treasurechest::tf::tokenfactory::TokenFactoryType;

use crate::contract::{execute, instantiate};
use crate::state::BIDDING_BALANCE;
use crate::ContractError;

pub struct AuctionQuerier {
    bank: BankQuerier,
}

impl AuctionQuerier {
    pub fn new() -> AuctionQuerier {
        AuctionQuerier {
            bank: BankQuerier::new(&[]),
        }
    }
}

impl Querier for AuctionQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> cosmwasm_std::QuerierResult {
        let request: QueryRequest<Empty> = from_json(&Binary::from(bin_request)).unwrap();
        match request {
            QueryRequest::Stargate {
                path,
                data: _,
            } => match path.as_str() {
                "/injective.auction.v1beta1.QueryCurrentAuctionBasketRequest" => {
                    Ok(CwContractResult::Ok(
                        to_json_binary(&QueryCurrentAuctionBasketResponse {
                            amount: vec![Coin {
                                denom: "uatom".to_string(),
                                amount: "10000".to_string(),
                            }],
                            auction_round: Some(1),
                            // simulates now + 7 days in seconds
                            auction_closing_time: Some(1_571_797_419 + 7 * 86_400),
                            highest_bidder: Some("highest_bidder".to_string()),
                            highest_bid_amount: Some("20000".to_string()),
                        })
                        .unwrap(),
                    ))
                    .into()
                },
                &_ => QuerierResult::Err(cosmwasm_std::SystemError::UnsupportedRequest {
                    kind: format!("Unmocked stargate query path: {path:?}"),
                }),
            },
            QueryRequest::Bank(query) => self.bank.query(&query),
            _ => QuerierResult::Err(cosmwasm_std::SystemError::UnsupportedRequest {
                kind: format!("Unmocked query type: {request:?}"),
            }),
        }
    }
}

pub fn mock_deps_with_querier(
    _info: &MessageInfo,
) -> OwnedDeps<MockStorage, MockApi, AuctionQuerier, Empty> {
    let deps = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: AuctionQuerier::new(),
        custom_query_type: PhantomData,
    };
    deps
}

pub fn init() -> (OwnedDeps<MemoryStorage, MockApi, AuctionQuerier>, Env) {
    let info = mock_info("instantiator", &coins(100, "denom"));
    let mut deps = mock_deps_with_querier(&info);
    let env = mock_env();

    let msg = InstantiateMsg {
        native_denom: "native_denom".to_string(),
        token_factory_type: TokenFactoryType::Injective,
        rewards_fee: Decimal::percent(10),
        rewards_fee_addr: "rewards_addr".to_string(),
        whitelisted_addresses: vec!["bot".to_string()],
        min_next_bid_increment_rate: Decimal::from_ratio(25u128, 10_000u128),
        treasury_chest_code_id: 1,
        min_return: Decimal::percent(5),
    };
    let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![attr("action", "instantiate"), attr("auction_round", "1"), attr("lp_subdenom", "1"),]
    );

    assert_eq!(
        res.messages[0].msg,
        TokenFactoryType::Injective.create_denom(env.contract.address.clone(), "1")
    );

    assert_eq!(BIDDING_BALANCE.load(&deps.storage).unwrap(), Uint128::zero());

    (deps, env)
}

#[test]
pub fn user_joins_pool() {
    let (mut deps, env) = init();

    let info = mock_info("robinho", &coins(100, "native_denom"));
    let msg = ExecuteMsg::JoinPool {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

    // contracts mints 100 lp tokens to itself. Subdenom is 1 as it's the first auction
    assert_eq!(
        res.messages[0].msg,
        TokenFactoryType::Injective
            .mint(env.contract.address.clone(), "1", Uint128::from(100u128),)
    );

    // contract sends 100 lp tokens to the user
    assert_eq!(
        res.messages[1].msg,
        BankMsg::Send {
            to_address: "robinho".to_string(),
            amount: coins(100, format!("factory/{}/{}", env.contract.address, 1)),
        }
        .into()
    );

    // contract calls try_bid on itself
    assert_eq!(
        res.messages[2].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::TryBid {
                auction_round: 1,
                basket_value: Uint128::from(10_000u128),
            })
            .unwrap(),
            funds: vec![],
        })
    );

    // checking attributes are fine
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "join_pool"),
            attr("auction_round", "1"),
            attr("sender", "robinho"),
            attr("bid_amount", "100"),
        ]
    );

    // bidding balance should now be 100
    assert_eq!(BIDDING_BALANCE.load(&deps.storage).unwrap(), Uint128::from(100u128));
}

#[test]
fn user_exit_pool_works() {
    let (mut deps, env) = init();

    let info = mock_info("robinho", &coins(100, "native_denom"));
    let msg = ExecuteMsg::JoinPool {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let _ = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

    let info = mock_info("robinho", &coins(100, format!("factory/{}/1", env.contract.address)));
    let msg = ExecuteMsg::ExitPool {};

    let res = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

    // contract burns 100 lp tokens from the user
    assert_eq!(
        res.messages[0].msg,
        TokenFactoryType::Injective.burn(
            env.contract.address.clone(),
            format!("factory/{}/1", env.contract.address).as_str(),
            Uint128::from(100u128),
        )
    );

    // contract returns 100 native_denom to the user
    assert_eq!(
        res.messages[1].msg,
        BankMsg::Send {
            to_address: "robinho".to_string(),
            amount: coins(100, "native_denom"),
        }
        .into()
    );

    // checking attributes are fine
    assert_eq!(res.attributes, vec![attr("action", "exit_pool")]);
}

#[test]
fn user_exit_pool_fails() {
    let (mut deps, mut env) = init();

    let info = mock_info("robinho", &coins(100, "native_denom"));
    let msg = ExecuteMsg::JoinPool {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let _ = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

    let info = mock_info("robinho", &coins(100, format!("factory/{}/1", env.contract.address)));
    let msg = ExecuteMsg::ExitPool {};

    // move time to 6 days later (1+ day before auction ends)
    env.block.time = env.block.time.plus_seconds(6 * 86_400 + 1);

    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap_err();

    // contract burns 100 lp tokens from the user
    assert_eq!(res, ContractError::PooledAuctionLocked {});

    // move time one more day, should be able to exit now
    env.block.time = env.block.time.plus_seconds(86_400);

    let res = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages[0].msg,
        TokenFactoryType::Injective.burn(
            env.contract.address.clone(),
            format!("factory/{}/1", env.contract.address).as_str(),
            Uint128::from(100u128),
        )
    );

    assert_eq!(
        res.messages[1].msg,
        BankMsg::Send {
            to_address: "robinho".to_string(),
            amount: coins(100, "native_denom"),
        }
        .into()
    );

    assert_eq!(res.attributes, vec![attr("action", "exit_pool")]);
}
