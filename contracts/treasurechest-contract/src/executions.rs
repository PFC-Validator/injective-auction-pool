use std::{ops::Mul, str::FromStr};
use std::collections::HashMap;
use std::ops::Sub;

const DEFAULT_SIZE:u32 = 20;
const MIN_TICKETS:u128 = 2;

use cosmwasm_std::{Addr, BalanceResponse, BankMsg, BankQuery, Coin, CosmosMsg, DepsMut, Env, Event, MessageInfo, Order,  Response, StdResult, Uint128};
use cosmwasm_std::QueryRequest::Bank;
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

pub fn return_dust(deps: DepsMut, env: Env, sender: Addr, limit: Option<u32>) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &sender)?;
    let config = CONFIG.load(deps.storage)?;

    let denom_total = deps.querier.query::<BalanceResponse>(&Bank(BankQuery::Supply {denom: config.denom.clone()}))?;
    if config.burn_it {
        if denom_total .amount.amount.u128()> MIN_TICKETS {
            return Err(ContractError::TicketsOutstanding(denom_total.amount.amount.u128(),MIN_TICKETS))
        }
    } else {
        let ticket_balance = deps.querier.query_balance(env.contract.address.clone(),config.denom.clone())?;
        let outstanding = denom_total.amount.amount.sub(ticket_balance.amount);
        if outstanding.u128() > MIN_TICKETS {
            return Err(ContractError::TicketsOutstanding(outstanding.u128(),MIN_TICKETS))
        }
    }

    let balances = deps
        .querier
        .query_all_balances(env.contract.address)?
        .into_iter()
        .filter(|x| x.denom != config.denom).map(|coin| (coin.denom,coin.amount))
        .collect::<HashMap<String,Uint128>>();
    let mut balances_out = vec![];

    let rewards = TOTAL_REWARDS.range(deps.storage, None,None,Order::Ascending).take(limit.unwrap_or(DEFAULT_SIZE).try_into()?).collect::<StdResult<Vec<_>>>()?;
    for reward in rewards {
        let reward_amt = reward.1.to_uint_floor();
        if let Some(token_balance) = balances.get( &reward.0) {
            if &reward_amt  > token_balance{
                TOTAL_REWARDS.remove(deps.storage, reward.0.clone());
                balances_out.push(Coin{denom: reward.0, amount: token_balance.clone()})
            }
        }
    }
    // balances_out should only contain the dust now.
    // TOTAL rewards should no longer show that token
    if balances_out.is_empty() {
       return Ok(Response::new().add_attribute("action", "treasurechest/return_dust").add_attribute("dust","no-dust"))
    }

    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: sender.to_string(),
        amount: balances_out,
    });
    Ok(Response::new().add_attribute("action", "treasurechest/return_dust").add_message(msg))
}
