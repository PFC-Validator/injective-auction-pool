use cosmwasm_std::{
    coins, ensure, to_json_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    WasmMsg,
};

use injective_auction::auction_pool::ExecuteMsg::TryBid;
use treasurechest::tf::tokenfactory::TokenFactoryType;

use crate::helpers::query_current_auction;
use crate::ContractError;

const INJ_DENOM: &str = "inj";
const DAY_IN_SECONDS: u64 = 86400;

/// Joins the pool
pub(crate) fn join_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    auction_round: u64,
) -> Result<Response, ContractError> {
    //todo ?Will reject funds once pool is above the current reward pool price?)

    cw_utils::must_pay(&info, INJ_DENOM)?;

    let coin = info.funds[0].clone();

    let current_auction_round = query_current_auction(deps.as_ref())?
        .auction_round
        .ok_or(ContractError::CurrentAuctionQueryError)?;

    ensure!(
        current_auction_round == auction_round,
        ContractError::InvalidAuctionRound {
            current_auction_round,
            auction_round
        }
    );

    let mut messages = vec![];

    // mint the lp token and send it to the user
    messages.push(TokenFactoryType::Injective.mint(
        env.contract.address.clone(),
        auction_round.to_string().as_str(),
        coin.amount,
    ));

    let lp_denom = format!("factory/{}/{}", env.contract.address, auction_round);
    messages.push(
        BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: lp_denom,
                amount: coin.amount,
            }],
        }
        .into(),
    );

    // try to bid on the auction if possible
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&TryBid {})?,
        funds: vec![],
    }));

    Ok(Response::default().add_messages(messages).add_attributes(vec![
        ("action", "join_pool".to_string()),
        ("auction_round", auction_round.to_string()),
        ("sender", info.sender.to_string()),
        ("bid_amount", coin.amount.to_string()),
    ]))
}

/// Exits the pool if the time is before T-1 day from the end of the auction.
pub(crate) fn exit_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let current_auction_round_response = query_current_auction(deps.as_ref())?;

    let lp_denom = format!(
        "factory/{}/{}",
        env.contract.address,
        current_auction_round_response
            .auction_round
            .ok_or(ContractError::CurrentAuctionQueryError)?
    );
    cw_utils::must_pay(&info, lp_denom.as_str())?;

    ensure!(
        DAY_IN_SECONDS
            > current_auction_round_response
                .auction_closing_time
                .ok_or(ContractError::CurrentAuctionQueryError)?
                .saturating_sub(env.block.time.seconds()),
        ContractError::PooledAuctionLocked
    );

    let mut messages = vec![];

    let coin = info.funds[0].clone();

    // burn the lp token and send the inj back to the user
    messages.push(TokenFactoryType::Injective.burn(
        env.contract.address.clone(),
        lp_denom.as_str(),
        coin.amount,
    ));
    messages.push(
        BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: coins(coin.amount.u128(), INJ_DENOM),
        }
        .into(),
    );

    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![("action", "exit_pool".to_string())]))
}

pub(crate) fn try_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    Ok(Response::default().add_attributes(vec![("action", "try_bid".to_string())]))
}
