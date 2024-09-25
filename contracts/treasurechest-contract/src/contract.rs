use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Coin, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use treasurechest::{
    chest::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    errors::ContractError,
    tf::tokenfactory::TokenFactoryType,
};

use crate::executions::withdraw;
use crate::{
    executions::{change_token_factory, return_dust},
    queries::{query_config, query_state},
    state::{Config, CONFIG, TOTAL_REWARDS},
};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "pfc-treasurechest";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(msg.owner.as_str()))?;

    let token_factory_type = TokenFactoryType::from_str(&msg.token_factory)
        .map_err(|_| ContractError::TokenFactoryTypeInvalid(msg.token_factory))?;

    let supply = deps.querier.query_supply(msg.denom.clone())?.amount;
    let split_rewards = split_reward_by_supply(info.funds, supply);

    TOTAL_REWARDS.clear(deps.storage);
    for entry in split_rewards {
        TOTAL_REWARDS.save(deps.storage, entry.0, &entry.1)?;
    }
    CONFIG.save(
        deps.storage,
        &Config {
            notes: msg.notes,
            denom: msg.denom.clone(),
            token_factory_type: token_factory_type.clone(),
            burn_it: msg.burn_it.unwrap_or(true),
        },
    )?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Withdraw {} => withdraw(deps, env, info),

        ExecuteMsg::UpdateOwnership(action) => {
            cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::default())
        },
        ExecuteMsg::ChangeTokenFactory {
            token_factory_type,
        } => change_token_factory(deps, info.sender, &token_factory_type),
        ExecuteMsg::ReturnDust {limit} => return_dust(deps, env, info.sender, limit),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let result = match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::State {} => to_json_binary(&query_state(deps, env)?),
        QueryMsg::Ownership {} => {
            let ownership = cw_ownable::get_ownership(deps.storage)?;
            to_json_binary(&ownership)
        },
    }?;

    Ok(result)
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;

    match contract_version.contract.as_ref() {
        #[allow(clippy::single_match)]
        "pfc-treasurechest" => {},
        _ => {
            return Err(ContractError::MigrationError {
                current_name: contract_version.contract,
                current_version: contract_version.version,
            });
        },
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

pub fn split_reward_by_supply(total: Vec<Coin>, total_supply: Uint128) -> HashMap<String, Decimal> {
    let mut single: HashMap<String, Decimal> = Default::default();
    for coin in total {
        let ratio: Decimal = Decimal::new(coin.amount) / Decimal::new(total_supply);
        single.insert(coin.denom, ratio);
    }
    single
}
