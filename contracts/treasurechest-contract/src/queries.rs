use cosmwasm_std::{Deps, Env, Order, StdResult};
use treasurechest::{
    chest::{ConfigResponse, DenomBalance, StateResponse},
    errors::ContractError,
};

use crate::state::{CONFIG, TOTAL_REWARDS};

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let owner = cw_ownable::get_ownership(deps.storage)?;
    let resp = ConfigResponse {
        denom: config.denom.to_string(),
        notes: config.notes.to_string(),
        owner: owner.owner.map(|x| x.to_string()).unwrap_or_default(),
        token_factory_type: config.token_factory_type,
        burn_it: config.burn_it,
    };

    Ok(resp)
}

pub fn query_state(deps: Deps, env: Env) -> Result<StateResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let balance = deps.querier.query_balance(env.contract.address, config.denom.clone())?;
    let counters = TOTAL_REWARDS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|f| {
            f.map(|x| DenomBalance {
                amount: x.1,
                denom: x.0,
            })
        })
        .collect::<StdResult<Vec<DenomBalance>>>()?;

    let supply = deps.querier.query_supply(config.denom)?;
    Ok(StateResponse {
        denom: supply.denom,
        holding: balance.amount,
        outstanding: supply.amount - balance.amount,
        rewards_per_one_token: counters,
    })
}
