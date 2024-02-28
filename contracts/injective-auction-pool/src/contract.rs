use cosmwasm_std::{entry_point, Addr};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use injective_auction::auction_pool::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::error::ContractError;
use crate::executions;
use crate::helpers::{query_current_auction, validate_rewards_fee};
use crate::state::{CONFIG, CURRENT_AUCTION_NUMBER};

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

    let whitelisted_addresses: Vec<Addr> = msg
        .bot_address
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<Result<_, _>>()?;

    CONFIG.save(
        deps.storage,
        &Config {
            rewards_fee: validate_rewards_fee(msg.rewards_fee)?,
            rewards_fee_addr: deps.api.addr_validate(&msg.rewards_fee_addr)?,
            whitelisted_addresses,
        },
    )?;

    let current_auction_round = query_current_auction(deps.as_ref())?
        .auction_round
        .ok_or(ContractError::CurrentAuctionQueryError)?;
    CURRENT_AUCTION_NUMBER.save(deps.storage, &current_auction_round)?;

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
        ExecuteMsg::SettleAuction {} => {
            unimplemented!()
        },
    }
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}
