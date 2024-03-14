use cosmwasm_std::{entry_point, to_json_binary, Addr};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use injective_auction::auction_pool::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::error::ContractError;
use crate::executions::{self, settle_auction};
use crate::helpers::{new_auction_round, validate_percentage};
use crate::queries;
use crate::state::CONFIG;

const CONTRACT_NAME: &str = "crates.io:injective-auction-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner.unwrap_or(info.sender.to_string()))?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(owner.to_string().as_str()))?;

    // Ensure that the contract is funded with at least the minimum balance
    let amount = cw_utils::must_pay(&info, &msg.native_denom)?;
    if amount < msg.min_balance {
        return Err(ContractError::InsufficientFunds {
            native_denom: msg.native_denom,
            min_balance: msg.min_balance,
        });
    }

    let whitelisted_addresses = msg
        .whitelisted_addresses
        .iter()
        .map(|addr| deps.api.addr_validate(addr))
        .collect::<Result<Vec<Addr>, _>>()?;

    CONFIG.save(
        deps.storage,
        &Config {
            native_denom: msg.native_denom,
            min_balance: msg.min_balance,
            token_factory_type: msg.token_factory_type.clone(),
            rewards_fee: validate_percentage(msg.rewards_fee)?,
            rewards_fee_addr: deps.api.addr_validate(&msg.rewards_fee_addr)?,
            whitelisted_addresses,
            min_next_bid_increment_rate: validate_percentage(msg.min_next_bid_increment_rate)?,
            treasury_chest_code_id: msg.treasury_chest_code_id,
            min_return: validate_percentage(msg.min_return)?,
        },
    )?;

    let (messages, attributes) = new_auction_round(deps, &env, info, None, None)?;

    Ok(Response::default()
        .add_messages(messages)
        .add_attribute("action", "instantiate")
        .add_attributes(attributes))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            rewards_fee,
            rewards_fee_addr,
            whitelist_addresses,
            min_next_bid_increment_rate,
            min_return,
        } => executions::update_config(
            deps,
            env,
            info,
            rewards_fee,
            rewards_fee_addr,
            whitelist_addresses,
            min_next_bid_increment_rate,
            min_return,
        ),
        ExecuteMsg::UpdateOwnership(action) => {
            cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::default())
        },
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => queries::query_config(deps),
        QueryMsg::Ownership {} => {
            let ownership = cw_ownable::get_ownership(deps.storage)?;
            to_json_binary(&ownership)
        },
        QueryMsg::TreasureChestContracts {
            start_after,
            limit,
        } => queries::query_treasure_chest_contracts(deps, start_after, limit),
        QueryMsg::BiddingBalance {} => queries::query_bidding_balance(deps),
    }
}
