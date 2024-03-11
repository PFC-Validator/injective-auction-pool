use cosmwasm_std::testing::{
    mock_env, mock_info, BankQuerier, MockApi, MockStorage, MOCK_CONTRACT_ADDR,
};
use cosmwasm_std::{
    attr, coin, coins, from_json, to_json_binary, Addr, BankMsg, Binary,
    ContractResult as CwContractResult, CosmosMsg, Decimal, Empty, Env, MemoryStorage, MessageInfo,
    OwnedDeps, Querier, QuerierResult, QueryRequest, Uint128, WasmMsg,
};
use injective_auction::auction::{Coin, MsgBid, QueryCurrentAuctionBasketResponse};
use injective_auction::auction_pool::{ExecuteMsg, InstantiateMsg};
use prost::Message;
use std::marker::PhantomData;
use treasurechest::tf::tokenfactory::TokenFactoryType;

use crate::contract::{execute, instantiate};
use crate::state::{BIDDING_BALANCE, CONFIG};
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
        owner: Some("owner".to_string()),
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
fn update_config() {
    let (mut deps, env) = init();

    // update config as non-owner should fail
    let info = mock_info("not_owner", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        rewards_fee: None,
        rewards_fee_addr: None,
        whitelist_addresses: None,
        min_next_bid_increment_rate: None,
        min_return: None,
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // update some of the config fields as owner should work
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("new_owner".to_string()),
        rewards_fee: Some(Decimal::percent(20)),
        rewards_fee_addr: Some("new_rewards_addr".to_string()),
        whitelist_addresses: Some(vec!["new_bot".to_string()]),
        min_next_bid_increment_rate: Some(Decimal::percent(10)),
        min_return: Some(Decimal::percent(10)),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_config"),
            attr("owner", "new_owner"),
            attr("native_denom", "native_denom"),
            attr("token_factory_type", "Injective"),
            attr("rewards_fee", "0.2"),
            attr("rewards_fee_addr", "new_rewards_addr"),
            attr("whitelisted_addresses", "new_bot"),
            attr("min_next_bid_increment_rate", "0.1"),
            attr("treasury_chest_code_id", "1"),
            attr("min_return", "0.1"),
        ]
    );

    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.owner, Addr::unchecked("new_owner"));
    assert_eq!(config.rewards_fee, Decimal::percent(20));
    assert_eq!(config.rewards_fee_addr, "new_rewards_addr".to_string());
    assert_eq!(config.whitelisted_addresses, vec!["new_bot".to_string()]);
    assert_eq!(config.min_next_bid_increment_rate, Decimal::percent(10));
    assert_eq!(config.min_return, Decimal::percent(10));
}

#[test]
pub fn join_pool_works() {
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
            amount: coins(100, format!("factory/{}/{}", MOCK_CONTRACT_ADDR, 1)),
        }
        .into()
    );

    // contract calls try_bid on itself
    assert_eq!(
        res.messages[2].msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_CONTRACT_ADDR.to_string(),
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
fn join_pool_fails() {
    let (mut deps, env) = init();

    // join pool with wrong denom should fail
    let info = mock_info("robinho", &coins(100, "wrong_denom"));
    let msg = ExecuteMsg::JoinPool {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::PaymentError(cw_utils::PaymentError::MissingDenom(
            "native_denom".to_string()
        ))
    );

    // join pool without funds should fail
    let info = mock_info("robinho", &[]);
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::PaymentError(cw_utils::PaymentError::NoFunds {}));

    // join pool sending 2 different denoms should fail
    let info = mock_info("robinho", &vec![coin(100, "native_denom"), coin(100, "wrong_denom")]);
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::PaymentError(cw_utils::PaymentError::MultipleDenoms {}));

    // joining the wrong auction round should fail
    let info = mock_info("robinho", &coins(100, "native_denom"));
    let msg = ExecuteMsg::JoinPool {
        auction_round: 2,
        basket_value: Uint128::from(10_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::InvalidAuctionRound {
            current_auction_round: 1,
            auction_round: 2
        }
    );
}

#[test]
fn exit_pool_works() {
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
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            format!("factory/{}/1", MOCK_CONTRACT_ADDR).as_str(),
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
fn exit_pool_fails() {
    let (mut deps, mut env) = init();

    let info = mock_info("robinho", &coins(100, "native_denom"));
    let msg = ExecuteMsg::JoinPool {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let _ = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg).unwrap();

    // exit pool with wrong denom should fail
    let msg = ExecuteMsg::ExitPool {};
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::PaymentError(cw_utils::PaymentError::MissingDenom(format!(
            "factory/{MOCK_CONTRACT_ADDR}/1",
        )))
    );

    // exit pool without funds should fail
    let info = mock_info("robinho", &[]);
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::PaymentError(cw_utils::PaymentError::NoFunds {}));

    // exit pool in T-1 day should fail
    let info = mock_info("robinho", &coins(100, format!("factory/{}/1", env.contract.address)));
    env.block.time = env.block.time.plus_seconds(6 * 86_400 + 1);
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::PooledAuctionLocked {});

    // exit pool after T-1 day should work now
    env.block.time = env.block.time.plus_seconds(86_400);

    let res = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.messages[0].msg,
        TokenFactoryType::Injective.burn(
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            format!("factory/{}/1", MOCK_CONTRACT_ADDR).as_str(),
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

#[test]
fn try_bid_works() {
    let (mut deps, mut env) = init();

    // try bid with no previous bids should not fail but won't bid either
    let info = mock_info("bot", &[]);
    let msg = ExecuteMsg::TryBid {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(res.messages.len(), 0);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "did_not_bid"),
            attr("reason", "minimum_allowed_bid_is_higher_than_bidding_balance")
        ]
    );

    // join pool with one user & enough funds to be able to outbid default highest bid (20000)
    let info = mock_info("robinho", &coins(30_000, "native_denom"));
    let msg = ExecuteMsg::JoinPool {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let _ = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

    let info = mock_info("bot", &[]);
    let msg = ExecuteMsg::TryBid {
        auction_round: 1,
        basket_value: Uint128::from(100_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap();

    // check the stargate bid message is correct, should only bid minimum allowed bid amount
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Stargate {
            type_url: "/injective.auction.v1beta1.MsgBid".to_string(),
            value: {
                let msg = MsgBid {
                    sender: env.contract.address.to_string(),
                    bid_amount: Some(injective_auction::auction::Coin {
                        denom: "native_denom".to_string(),
                        amount: "20051".to_string(),
                    }),
                    round: 1,
                };
                Binary(msg.encode_to_vec())
            },
        }
    );
    assert_eq!(res.attributes, vec![attr("action", "try_bid"), attr("amount", "20051"),]);

    // try bid on a basket value that is lower than the highest bid should not fail but not bid either
    let info = mock_info("bot", &[]);
    let msg = ExecuteMsg::TryBid {
        auction_round: 1,
        basket_value: Uint128::from(5_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(res.messages.len(), 0);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "did_not_bid"),
            attr("reason", "basket_value_is_not_worth_bidding_for")
        ]
    );

    // try bid as the highest_bidder should not fail but won't bid either
    env.contract.address = Addr::unchecked("highest_bidder");
    let info = mock_info("highest_bidder", &[]);
    let msg = ExecuteMsg::TryBid {
        auction_round: 1,
        basket_value: Uint128::from(100_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap();
    assert_eq!(res.messages.len(), 0);
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "did_not_bid"),
            attr("reason", "contract_is_already_the_highest_bidder")
        ]
    );
}

#[test]
fn try_bid_fails() {
    let (mut deps, env) = init();

    // join pool with one user & enough funds to be able to outbid default highest bid (20000)
    let info = mock_info("robinho", &coins(30_000, "native_denom"));
    let msg = ExecuteMsg::JoinPool {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let _ = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

    // try bid from non-whitelisted address should fail
    let info = mock_info("non_whitelisted", &[]);
    let msg = ExecuteMsg::TryBid {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg);
    assert_eq!(res.unwrap_err(), ContractError::Unauthorized {});

    // try_bid with wrong auction round should fail
    let info = mock_info("bot", &[]);
    let msg = ExecuteMsg::TryBid {
        auction_round: 2,
        basket_value: Uint128::from(10_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::InvalidAuctionRound {
            current_auction_round: 1,
            auction_round: 2
        }
    );

    // try_bid with funds should fail
    let info = mock_info("bot", &coins(20_000, "native_denom"));
    let msg = ExecuteMsg::TryBid {
        auction_round: 1,
        basket_value: Uint128::from(10_000u128),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::PaymentError(cw_utils::PaymentError::NonPayable {}));
}
