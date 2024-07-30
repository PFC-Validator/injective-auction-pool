use std::marker::PhantomData;

use cosmwasm_std::{
    attr, coin, coins, from_json,
    testing::{mock_env, mock_info, BankQuerier, MockApi, MockStorage, MOCK_CONTRACT_ADDR},
    to_json_binary, Addr, BankMsg, Binary, CodeInfoResponse, ContractResult as CwContractResult,
    CosmosMsg, Decimal, Empty, Env, HexBinary, Int64, MemoryStorage, MessageInfo, OwnedDeps,
    Querier, QuerierResult, QueryRequest, Uint128, Uint64, WasmMsg, WasmQuery,
};
use cw_ownable::Ownership;
use injective_auction::auction_pool::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, WhitelistedAddressesResponse,
};
use injective_std::types::cosmos::base::v1beta1::Coin;
use injective_std::types::injective::auction::v1beta1::MsgBid;
use prost::Message;
use treasurechest::tf::tokenfactory::TokenFactoryType;

use crate::{
    contract::{execute, instantiate, query},
    state::{BIDDING_BALANCE, FUNDS_LOCKED},
    ContractError,
};

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
        let request: QueryRequest<Empty> = from_json(Binary::from(bin_request)).unwrap();
        match request {
            QueryRequest::Stargate {
                path,
                data: _,
            } => match path.as_str() {
                "/injective.auction.v1beta1.Query/CurrentAuctionBasket" => {
                    Ok(CwContractResult::Ok(
                        to_json_binary(&crate::state::QueryCurrentAuctionBasketResponse {
                            amount: vec![cosmwasm_std::Coin {
                                denom: "uatom".to_string(),
                                amount: Uint128::new(10000u128),
                            }],
                            auction_round: Uint64::one(),
                            // simulates now + 7 days in seconds
                            auction_closing_time: Int64::new(1_571_797_419 + 7 * 86_400),
                            highest_bidder: "highest_bidder".to_string(),
                            highest_bid_amount: Uint128::new(20000u128),
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
            QueryRequest::Wasm(WasmQuery::CodeInfo {
                code_id,
            }) => Ok(CwContractResult::Ok(
                to_json_binary(&CodeInfoResponse::new(
                    code_id,
                    Addr::unchecked("creator").to_string(),
                    HexBinary::from_hex(
                        "13a1fc994cc6d1c81b746ee0c0ff6f90043875e0bf1d9be6b7d779fc978dc2a5",
                    )
                    .unwrap(),
                ))
                .unwrap(),
            ))
            .into(),
            _ => QuerierResult::Err(cosmwasm_std::SystemError::UnsupportedRequest {
                kind: format!("Unmocked query type: {request:?}"),
            }),
        }
    }
}

pub fn mock_deps_with_querier(
    _info: &MessageInfo,
) -> OwnedDeps<MockStorage, MockApi, AuctionQuerier, Empty> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: AuctionQuerier::new(),
        custom_query_type: PhantomData,
    }
}

pub fn init() -> (OwnedDeps<MemoryStorage, MockApi, AuctionQuerier>, Env) {
    let info = mock_info("instantiator", &coins(2, "native_denom"));
    let mut deps = mock_deps_with_querier(&info);
    let env = mock_env();

    let msg = InstantiateMsg {
        owner: Some("owner".to_string()),
        native_denom: "native_denom".to_string(),
        min_balance: Uint128::from(2u128),
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
        vec![
            attr("action", "instantiate"),
            attr("new_auction_round", "1"),
            attr("lp_subdenom", "auction.0"),
        ]
    );

    assert_eq!(
        res.messages[0].msg,
        TokenFactoryType::Injective.create_denom(env.contract.address.clone(), "auction.0")
    );

    assert_eq!(BIDDING_BALANCE.load(&deps.storage).unwrap(), Uint128::zero());

    let msg = QueryMsg::WhitelistedAddresses {};
    let res: WhitelistedAddressesResponse =
        from_json(query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res.addresses, vec!["bot".to_string()]);

    (deps, env)
}

#[test]
fn update_config() {
    let (mut deps, env) = init();

    // update config as non-owner should fail
    let info = mock_info("not_owner", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        rewards_fee: None,
        rewards_fee_addr: None,
        min_next_bid_increment_rate: None,
        min_return: None,
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Ownership(cw_ownable::OwnershipError::NotOwner));

    // update some of the config fields as owner should work
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        rewards_fee: Some(Decimal::percent(20)),
        rewards_fee_addr: Some("new_rewards_addr".to_string()),
        min_next_bid_increment_rate: Some(Decimal::percent(10)),
        min_return: Some(Decimal::percent(10)),
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_config"),
            attr("native_denom", "native_denom"),
            attr("token_factory_type", "Injective"),
            attr("rewards_fee", "0.2"),
            attr("rewards_fee_addr", "new_rewards_addr"),
            attr("min_next_bid_increment_rate", "0.1"),
            attr("treasury_chest_code_id", "1"),
            attr("min_return", "0.1"),
        ]
    );

    // query the config to check if it was updated
    let msg = QueryMsg::Config {};
    let res: ConfigResponse = from_json(query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    let config = res.config;
    assert_eq!(config.rewards_fee, Decimal::percent(20));
    assert_eq!(config.rewards_fee_addr, "new_rewards_addr".to_string());
    assert_eq!(config.min_next_bid_increment_rate, Decimal::percent(10));
    assert_eq!(config.min_return, Decimal::percent(10));
}

#[test]
fn update_ownership() {
    let (mut deps, env) = init();

    // update ownership as non-owner should fail
    let info = mock_info("not_owner", &[]);
    let msg = ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
        new_owner: "new_owner".to_string(),
        expiry: None,
    });
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Ownership(cw_ownable::OwnershipError::NotOwner));

    // update ownership as owner should work
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
        new_owner: "new_owner".to_string(),
        expiry: None,
    });
    let _ = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap();

    // ownership should not be updated until accepted
    let msg = QueryMsg::Ownership {};
    let res: Ownership<Addr> = from_json(query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res.owner.unwrap(), "owner");

    // accept ownership as new_owner should work
    let info = mock_info("new_owner", &[]);
    let msg = ExecuteMsg::UpdateOwnership(cw_ownable::Action::AcceptOwnership {});
    let _ = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap();

    // query the ownership to check if it was updated
    let msg = QueryMsg::Ownership {};
    let res: Ownership<Addr> = from_json(query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res.owner.unwrap(), "new_owner");
}

#[test]
fn add_whitelist_address() {
    let (mut deps, env) = init();

    // whitelist address as non-owner should fail
    let info = mock_info("not_owner", &[]);
    let msg = ExecuteMsg::UpdateWhiteListedAddresses {
        remove: vec![],
        add: vec!["new_whitelisted".to_string()],
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Ownership(cw_ownable::OwnershipError::NotOwner));

    // whitelist addresses as owner should work
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateWhiteListedAddresses {
        remove: vec![],
        add: vec!["new_whitelisted".to_string(), "another_whitelisted".to_string()],
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_whitelisted_addresses"),
            attr("added_address", "new_whitelisted"),
            attr("added_address", "another_whitelisted"),
        ]
    );

    // whitelist an already whitelisted address should fail
    let msg = ExecuteMsg::UpdateWhiteListedAddresses {
        remove: vec![],
        add: vec!["new_whitelisted".to_string()],
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::AddressAlreadyWhitelisted {
            address: "new_whitelisted".to_string()
        }
    );

    // query the whitelisted addresses to check if it was updated
    let msg = QueryMsg::WhitelistedAddresses {};
    let res: WhitelistedAddressesResponse =
        from_json(query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(
        res.addresses,
        vec!["another_whitelisted".to_string(), "bot".to_string(), "new_whitelisted".to_string()]
    );
}

#[test]
fn remove_whitelisted_address() {
    let (mut deps, env) = init();

    // add a whitelisted address as an owner whould work
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateWhiteListedAddresses {
        remove: vec![],
        add: vec!["new_whitelisted".to_string()],
    };
    let _ = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap();

    // remove whitelisted address as non-owner should fail
    let info = mock_info("not_owner", &[]);
    let msg = ExecuteMsg::UpdateWhiteListedAddresses {
        remove: vec!["bot".to_string()],
        add: vec![],
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Ownership(cw_ownable::OwnershipError::NotOwner));

    // remove whitelisted address as owner should work
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateWhiteListedAddresses {
        remove: vec!["bot".to_string()],
        add: vec![],
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "update_whitelisted_addresses"), attr("removed_address", "bot"),]
    );

    // remove a non-existing whitelisted address should fail
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::UpdateWhiteListedAddresses {
        remove: vec!["not_whitelisted_address".to_string()],
        add: vec![],
    };
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::AddressNotWhitelisted {
            address: "not_whitelisted_address".to_string()
        }
    );

    // query the whitelisted addresses to check if it was updated
    let msg = QueryMsg::WhitelistedAddresses {};
    let res: WhitelistedAddressesResponse =
        from_json(query(deps.as_ref(), env.clone(), msg).unwrap()).unwrap();
    assert_eq!(res.addresses, vec![Addr::unchecked("new_whitelisted").to_string()]);
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
        TokenFactoryType::Injective.mint(
            env.contract.address.clone(),
            format!("factory/{}/auction.0", env.contract.address).as_str(),
            Uint128::from(100u128),
        )
    );

    // contract sends 100 lp tokens to the user
    assert_eq!(
        res.messages[1].msg,
        BankMsg::Send {
            to_address: "robinho".to_string(),
            amount: coins(100, format!("factory/{}/{}", MOCK_CONTRACT_ADDR, "auction.0")),
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
    let info = mock_info("robinho", &[coin(100, "native_denom"), coin(100, "wrong_denom")]);
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

    let info =
        mock_info("robinho", &coins(100, format!("factory/{}/auction.0", env.contract.address)));
    let msg = ExecuteMsg::ExitPool {};

    let res = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

    // contract burns 100 lp tokens from the user
    assert_eq!(
        res.messages[0].msg,
        TokenFactoryType::Injective.burn(
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            format!("factory/{}/auction.0", MOCK_CONTRACT_ADDR).as_str(),
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
    let (mut deps, env) = init();

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
            "factory/{MOCK_CONTRACT_ADDR}/auction.0",
        )))
    );

    // exit pool without funds should fail
    let info = mock_info("robinho", &[]);
    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::PaymentError(cw_utils::PaymentError::NoFunds {}));

    // exit pool in T-1 day should work as the contract has not bid yet
    let info =
        mock_info("robinho", &coins(100, format!("factory/{}/auction.0", env.contract.address)));

    let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages[0].msg,
        TokenFactoryType::Injective.burn(
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            format!("factory/{}/auction.0", MOCK_CONTRACT_ADDR).as_str(),
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

    // exit pool after the contract bid should fail
    FUNDS_LOCKED.save(deps.as_mut().storage, &true).unwrap();
    let res = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::PooledAuctionLocked {});
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
                    bid_amount: Some(Coin {
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

    assert!(FUNDS_LOCKED.load(&deps.storage).unwrap());

    // try bid on a basket value that is lower than the highest bid should not fail but not bid
    // either
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

// TODO: to test settle auction, need to comment the line that checks if the auction round is valid
// on executions.rs
//
// use crate::state::{TREASURE_CHEST_CONTRACTS, UNSETTLED_AUCTION};
// #[test]
// fn settle_auction_as_loser_works() {
//     let (mut deps, mut env) = init();

//     // join pool with one user & enough funds to be able to outbid default highest bid (20000)
//     let info = mock_info("robinho", &coins(30_000, "native_denom"));
//     let msg = ExecuteMsg::JoinPool {
//         auction_round: 1,
//         basket_value: Uint128::from(25_000u128),
//     };
//     let _ = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

//     // mock the auction round to be 0 so the contract thinks the auction round is over
//     let mut unsettled_auction = UNSETTLED_AUCTION.load(deps.as_ref().storage).unwrap();
//     unsettled_auction.auction_round = 0;
//     UNSETTLED_AUCTION.save(deps.as_mut().storage, &unsettled_auction).unwrap();
//     env.block.time = env.block.time.plus_days(7);

//     // settle auction with contract not being the highest bidder should work
//     let info = mock_info("bot", &[]);
//     let msg = ExecuteMsg::SettleAuction {
//         auction_round: 1,
//         auction_winner: "highest_bidder".to_string(),
//         auction_winning_bid: Uint128::from(20000u128),
//     };
//     let res = execute(deps.as_mut().branch(), env.clone(), info.clone(), msg.clone()).unwrap();
//     assert_eq!(res.messages.len(), 0);
//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "settle_auction"),
//             attr("settled_auction_round", "0"),
//             attr("new_auction_round", "1"),
//         ]
//     );

//     let unsettled_auction = UNSETTLED_AUCTION.load(deps.as_ref().storage).unwrap();
//     assert_eq!(unsettled_auction.auction_round, 1);
//     assert_eq!(unsettled_auction.basket, vec![coin(10_000, "uatom")]);
//     assert_eq!(unsettled_auction.closing_time, 1_571_797_419 + 7 * 86_400);
//     assert_eq!(unsettled_auction.lp_subdenom, 0);

//     // funds should be released
//     assert!(!FUNDS_LOCKED.load(deps.as_ref().storage).unwrap());
// }

// #[test]
// fn settle_auction_as_winner_works() {
//     let (mut deps, mut env) = init();

//     // join pool with one user & enough funds to be able to outbid default highest bid (20000)
//     let info = mock_info("robinho", &coins(30_000, "native_denom"));
//     let msg = ExecuteMsg::JoinPool {
//         auction_round: 1,
//         basket_value: Uint128::from(25_000u128),
//     };
//     let _ = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

//     // mock the auction round to be 0 so the contract thinks the auction round is over
//     let mut unsettled_auction = UNSETTLED_AUCTION.load(deps.as_ref().storage).unwrap();
//     unsettled_auction.auction_round = 0;
//     unsettled_auction.lp_subdenom = 1;
//     UNSETTLED_AUCTION.save(deps.as_mut().storage, &unsettled_auction).unwrap();
//     env.block.time = env.block.time.plus_days(7);

//     // settle auction with highest bidder should work
//     let info = mock_info("bot", &[]);
//     let msg = ExecuteMsg::SettleAuction {
//         auction_round: 1,
//         auction_winner: "cosmos2contract".to_string(),
//         auction_winning_bid: Uint128::from(20000u128),
//     };
//     let res = execute(deps.as_mut().branch(), env.clone(), info, msg).unwrap();

//     // check the stargate settle auction message is correct
//     assert_eq!(
//         res.messages[0].msg,
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address: "rewards_addr".to_string(),
//             // 10% of the basket assets (10_000 uatom)
//             amount: coins(10_000 * 10 / 100, "uatom"),
//         })
//     );

//     assert_eq!(
//         res.messages[1].msg,
//         WasmMsg::Instantiate2 {
//             admin: Some("cosmos2contract".to_string()),
//             code_id: 1,
//             label: "Treasure chest for auction round 0".to_string(),
//             msg: to_json_binary(&treasurechest::chest::InstantiateMsg {
//                 denom: "native_denom".to_string(),
//                 owner: "cosmos2contract".to_string(),
//                 notes: "factory/cosmos2contract/auction.1".to_string(),
//                 token_factory: TokenFactoryType::Injective.to_string(),
//                 burn_it: Some(false)
//             })
//             .unwrap(),
//             funds: vec![coin(10_000, "native_denom"), coin(10_000 * 90 / 100, "uatom")],
//             salt: Binary::from(
//                 format!(
//                     "{}{}{}",
//                     unsettled_auction.auction_round,
//                     "bot".to_string(),
//                     env.block.height
//                 )
//                 .as_bytes()
//             ),
//         }
//         .into()
//     );

//     let treasure_chest_addr = TREASURE_CHEST_CONTRACTS.load(&deps.storage, 0).unwrap();

//     assert_eq!(
//         res.messages[2].msg,
//         TokenFactoryType::Injective.change_admin(
//             Addr::unchecked("cosmos2contract"),
//             "factory/cosmos2contract/auction.1",
//             treasure_chest_addr
//         )
//     );

//     assert_eq!(
//         res.messages[3].msg,
//         TokenFactoryType::Injective.create_denom(Addr::unchecked(MOCK_CONTRACT_ADDR),
// "auction.2")     );

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "settle_auction"),
//             attr("settled_auction_round", "0"),
//             attr("new_auction_round", "1"),
//             attr(
//                 "treasure_chest_address",
//                 // this is a mock address, as the checksum was invented
//                 "ED9963158CC851609F6BCFE30C3256F3471F11E3087F6DB5244B1FE5659757C4"
//             ),
//             attr("new_subdenom", "auction.2"),
//         ]
//     );

//     // funds should be released
//     assert!(!FUNDS_LOCKED.load(deps.as_ref().storage).unwrap());
// }
