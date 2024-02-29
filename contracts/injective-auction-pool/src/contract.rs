use cosmwasm_std::{entry_point, Addr, Decimal, Uint128};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use injective_auction::auction_pool::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::error::ContractError;
use crate::executions::{self, settle_auction};
use crate::helpers::{query_current_auction, validate_rewards_fee};
use crate::state::{BIDDING_BALANCE, CONFIG, CURRENT_AUCTION_ROUND};

const CONTRACT_NAME: &str = "crates.io:injective-auction-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let whitelisted_addresses = msg
        .whitelisted_addresses
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<Result<Vec<Addr>, _>>()?;

    let current_auction_round = query_current_auction(deps.as_ref())?
        .auction_round
        .ok_or(ContractError::CurrentAuctionQueryError)?;

    CURRENT_AUCTION_ROUND.save(deps.storage, &current_auction_round)?;

    if msg.min_bid_percentage > Decimal::percent(100) {
        return Err(ContractError::InvalidMaxBidPercentage);
    }

    CONFIG.save(
        deps.storage,
        &Config {
            rewards_fee: validate_rewards_fee(msg.rewards_fee)?,
            rewards_fee_addr: deps.api.addr_validate(&msg.rewards_fee_addr)?,
            whitelisted_addresses,
            min_bid_percentage: msg.min_bid_percentage,
        },
    )?;

    BIDDING_BALANCE.save(deps.storage, &Uint128::zero())?;

    //todo mint lp for current auction

    Ok(Response::default())
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
            auction_round: auction_id,
        } => executions::join_pool(deps, env, info, auction_id),
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
