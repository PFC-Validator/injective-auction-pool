use std::{ops::Mul, str::FromStr};

use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, Event, MessageInfo, Order, Response, StdResult,
    Uint128,
};
use treasurechest::{errors::ContractError, tf::tokenfactory::TokenFactoryType};

use crate::state::{CONFIG, TOTAL_REWARDS};

// withdraw rewards to executor
pub fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.funds.is_empty() {
        Err(ContractError::NeedTicketDenom(config.denom))
    } else if info.funds.len() != 1 {
        Err(ContractError::OnlyTicketDenom(config.denom))
    } else if let Some(tickets) = info.funds.first() {
        if config.denom != tickets.denom {
            Err(ContractError::OnlyTicketDenom(config.denom))
        } else {
            let mut msgs: Vec<CosmosMsg> = vec![];
            let to_send: Vec<Coin> = TOTAL_REWARDS
                .range(deps.storage, None, None, Order::Ascending)
                .map(|item| {
                    item.map(|chest| {
                        let amount = chest.1.mul(tickets.amount);
                        Coin::new(amount.into(), chest.0)
                    })
                })
                .collect::<StdResult<Vec<Coin>>>()?
                .into_iter()
                .filter(|x| x.amount > Uint128::zero())
                .collect::<Vec<Coin>>();
            let msg_send = CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: to_send,
            });
            msgs.push(msg_send);

            if config.burn_it {
                let msg_burn = config.token_factory_type.burn(
                    env.contract.address,
                    &tickets.denom,
                    tickets.amount,
                );
                msgs.push(msg_burn)
            }

            Ok(Response::new()
                .add_attributes(vec![("action", "treasurechest/withdraw")])
                .add_messages(msgs))
        }
    } else {
        Err(ContractError::OnlyTicketDenom(config.denom))
    }
}

pub fn change_token_factory(
    deps: DepsMut,
    sender: Addr,
    token_factory_type: &str,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &sender)?;
    let tf = TokenFactoryType::from_str(token_factory_type)
        .map_err(|_| ContractError::TokenFactoryTypeInvalid(token_factory_type.into()))?;
    CONFIG.update(deps.storage, |mut config| -> Result<_, ContractError> {
        config.token_factory_type = tf;
        Ok(config)
    })?;
    let event = Event::new("treasurechest/change_token_factory")
        .add_attribute("token_factory_type", token_factory_type);

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "treasurechest/change_token_factory"))
}

pub fn return_dust(deps: DepsMut, env: Env, sender: Addr) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &sender)?;
    let config = CONFIG.load(deps.storage)?;
    let balances = deps
        .querier
        .query_all_balances(env.contract.address)?
        .into_iter()
        .filter(|x| x.denom != config.denom)
        .collect::<Vec<Coin>>();
    let mut balances_out = vec![];

    for entry in balances {
        if let Some(one_amt) = TOTAL_REWARDS.may_load(deps.storage, entry.denom.clone())? {
            if one_amt.to_uint_floor() > entry.amount {
                TOTAL_REWARDS.remove(deps.storage, entry.denom.clone());
                balances_out.push(entry)
            }
        }
    }
    // balances_out should only contain the dust now.
    // TOTAL rewards should no longer show that token

    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: sender.to_string(),
        amount: balances_out,
    });
    Ok(Response::new().add_attribute("action", "treasurechest/return_dust").add_message(msg))
}
