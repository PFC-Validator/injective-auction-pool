use std::str::FromStr;

use cosmwasm_std::{entry_point, Addr, Coin, Uint128};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use injective_auction::auction_pool::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::error::ContractError;
use crate::executions::{self, settle_auction};
use crate::helpers::{query_current_auction, validate_percentage};
use crate::state::{Auction, BIDDING_BALANCE, CONFIG, UNSETTLED_AUCTION};

const CONTRACT_NAME: &str = "crates.io:injective-auction-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let whitelisted_addresses = msg
        .whitelisted_addresses
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<Result<Vec<Addr>, _>>()?;

    CONFIG.save(
        deps.storage,
        &Config {
            native_denom: msg.native_denom,
            token_factory_type: msg.token_factory_type.clone(),
            rewards_fee: validate_percentage(msg.rewards_fee)?,
            rewards_fee_addr: deps.api.addr_validate(&msg.rewards_fee_addr)?,
            whitelisted_addresses,
            min_next_bid_increment_rate: validate_percentage(msg.min_next_bid_increment_rate)?,
            treasury_chest_code_id: msg.treasury_chest_code_id,
            min_return: validate_percentage(msg.min_return)?,
        },
    )?;

    // fetch current auction details and save them in the contract state
    let current_auction_round_response = query_current_auction(deps.as_ref())?;

    let auction_round = current_auction_round_response
        .auction_round
        .ok_or(ContractError::CurrentAuctionQueryError)?;

    let basket = current_auction_round_response
        .amount
        .iter()
        .map(|coin| Coin {
            amount: Uint128::from_str(&coin.amount).expect("Failed to parse coin amount"),
            denom: coin.denom.clone(),
        })
        .collect();

    UNSETTLED_AUCTION.save(
        deps.storage,
        &Auction {
            basket,
            auction_round,
            lp_subdenom: 1,
            closing_time: current_auction_round_response.auction_closing_time(),
        },
    )?;

    BIDDING_BALANCE.save(deps.storage, &Uint128::zero())?;

    // create a new denom for the current auction round
    let msg = msg.token_factory_type.create_denom(env.contract.address.clone(), "1");

    Ok(Response::default()
        .add_message(msg)
        .add_attribute("action", "instantiate")
        .add_attribute("auction_round", auction_round.to_string())
        .add_attribute("lp_subdenom", "1"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::TryBid {
            auction_round,
            basket_value,
        } => executions::try_bid(deps, env, info, auction_round, basket_value),
        ExecuteMsg::JoinPool {
            auction_round,
            basket_value,
        } => executions::join_pool(deps, env, info, auction_round, basket_value),
        ExecuteMsg::ExitPool {} => executions::exit_pool(deps, env, info),
        ExecuteMsg::SettleAuction {
            auction_round,
            auction_winner,
            auction_winning_bid,
        } => settle_auction(deps, env, info, auction_round, auction_winner, auction_winning_bid),
    }
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}
