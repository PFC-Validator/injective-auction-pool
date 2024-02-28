use cosmwasm_std::{
    coins, ensure, to_json_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response,
    Uint128, WasmMsg,
};

use injective_auction::auction_pool::ExecuteMsg::TryBid;
use treasurechest::tf::tokenfactory::TokenFactoryType;

use crate::helpers::query_current_auction;
use crate::state::CONFIG;
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

    // TODO: define how this basket value is going to be calculated,
    // either passed as an argument to the function or queried from a DEX.
    // for now, we are using a dummy value of 0 to please the compiler
    let basket_value = Uint128::zero();

    // try to bid on the auction if possible
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&TryBid {
            auction_round,
            basket_value,
        })?,
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
    auction_round: u64,
    basket_value: Uint128,
) -> Result<Response, ContractError> {
    let current_auction_round_response = query_current_auction(deps.as_ref())?;

    // prevents the contract from bidding on the wrong auction round
    if auction_round
        != current_auction_round_response
            .auction_round
            .ok_or(ContractError::CurrentAuctionQueryError)?
    {
        return Err(ContractError::InvalidAuctionRound {
            current_auction_round: current_auction_round_response
                .auction_round
                .ok_or(ContractError::CurrentAuctionQueryError)?,
            auction_round,
        });
    }

    // only whitelist addresses or the contract itself can bid on the auction
    let config = CONFIG.load(deps.storage)?;
    if info.sender != env.contract.address || !config.whitelisted_addresses.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // prevents the contract from bidding if the contract is already the highest bidder
    let highest_bidder =
        current_auction_round_response.highest_bidder.ok_or(ContractError::NoBidsFound)?;
    if highest_bidder == env.contract.address {
        return Ok(Response::default().add_attribute("action", "already_highest_bidder"));
    }

    // prevents the contract from bidding if the the current bid is higher than the basket value
    let current_bid_amount: Uint128 = current_auction_round_response
        .highest_bid_amount
        .ok_or(ContractError::NoBidsFound)?
        .parse()?;

    if current_bid_amount >= basket_value {
        return Ok(Response::default().add_attribute("action", "bid_is_higher_than_basket_amount"));
    }

    // TODO: continue with the bidding process

    Ok(Response::default().add_attributes(vec![("action", "try_bid".to_string())]))
}
