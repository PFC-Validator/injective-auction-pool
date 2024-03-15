use crate::helpers::{new_auction_round, query_current_auction, validate_percentage};
use crate::state::{
    Whitelisted, BIDDING_BALANCE, CONFIG, UNSETTLED_AUCTION, WHITELISTED_ADDRESSES,
};
use crate::ContractError;
use cosmwasm_std::{
    attr, coins, to_json_binary, BankMsg, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response,
    Uint128, WasmMsg,
};
use injective_auction::auction::MsgBid;
use injective_auction::auction_pool::ExecuteMsg::TryBid;

const DAY_IN_SECONDS: u64 = 86400;

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    rewards_fee: Option<Decimal>,
    rewards_fee_addr: Option<String>,
    min_next_bid_increment_rate: Option<Decimal>,
    min_return: Option<Decimal>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    if let Some(rewards_fee) = rewards_fee {
        config.rewards_fee = validate_percentage(rewards_fee)?;
    }

    if let Some(rewards_fee_addr) = rewards_fee_addr {
        config.rewards_fee_addr = deps.api.addr_validate(&rewards_fee_addr)?;
    }

    if let Some(min_next_bid_increment_rate) = min_next_bid_increment_rate {
        config.min_next_bid_increment_rate = validate_percentage(min_next_bid_increment_rate)?;
    }

    if let Some(min_return) = min_return {
        config.min_return = validate_percentage(min_return)?;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "update_config")
        .add_attribute("native_denom", config.native_denom)
        .add_attribute("token_factory_type", config.token_factory_type.to_string())
        .add_attribute("rewards_fee", config.rewards_fee.to_string())
        .add_attribute("rewards_fee_addr", config.rewards_fee_addr.to_string())
        .add_attribute(
            "min_next_bid_increment_rate",
            config.min_next_bid_increment_rate.to_string(),
        )
        .add_attribute("treasury_chest_code_id", config.treasury_chest_code_id.to_string())
        .add_attribute("min_return", config.min_return.to_string()))
}

pub fn update_whitelisted_addresses(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    remove: Vec<String>,
    add: Vec<String>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut added = vec![];
    for addr in add.clone().into_iter() {
        let add_addr = deps.api.addr_validate(&addr)?;
        if !WHITELISTED_ADDRESSES.has(deps.storage, &add_addr) {
            WHITELISTED_ADDRESSES.save(deps.storage, &add_addr, &Whitelisted)?;
            added.push(attr("added_address", addr));
        } else {
            return Err(ContractError::AddressAlreadyWhitelisted {
                address: addr,
            });
        }
    }

    let mut removed = vec![];
    for addr in remove.clone().into_iter() {
        let remove_addr = deps.api.addr_validate(&addr)?;
        if WHITELISTED_ADDRESSES.has(deps.storage, &remove_addr) {
            WHITELISTED_ADDRESSES.remove(deps.storage, &remove_addr);
            removed.push(attr("removed_address", addr));
        } else {
            return Err(ContractError::AddressNotWhitelisted {
                address: addr,
            });
        }
    }

    Ok(Response::default()
        .add_attribute("action", "update_whitelisted_addresses")
        .add_attributes(removed)
        .add_attributes(added))
}

/// Joins the pool
pub(crate) fn join_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    auction_round: u64,
    basket_value: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let amount = cw_utils::must_pay(&info, &config.native_denom)?;

    let current_auction_round = query_current_auction(deps.as_ref())?
        .auction_round
        .ok_or(ContractError::CurrentAuctionQueryError)?;

    // prevents the user from joining the pool if the auction round is over
    if auction_round != current_auction_round {
        return Err(ContractError::InvalidAuctionRound {
            current_auction_round,
            auction_round,
        });
    }

    let mut messages = vec![];

    // mint the lp token and send it to the user
    let lp_subdenom = UNSETTLED_AUCTION.load(deps.storage)?.lp_subdenom;
    messages.push(config.token_factory_type.mint(
        env.contract.address.clone(),
        format!("auction.{}", lp_subdenom).as_str(),
        amount,
    ));

    // send the minted lp token to the user
    let lp_denom = format!("factory/{}/auction.{}", env.contract.address, lp_subdenom);
    messages.push(
        BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: coins(amount.into(), lp_denom),
        }
        .into(),
    );

    // increase the balance that can be used for bidding
    BIDDING_BALANCE
        .update::<_, ContractError>(deps.storage, |balance| Ok(balance.checked_add(amount)?))?;

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
        ("bid_amount", amount.to_string()),
    ]))
}

/// Exits the pool if the time is before T-1 day from the end of the auction.
pub(crate) fn exit_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let current_auction_round_response = query_current_auction(deps.as_ref())?;

    //make sure the user sends a correct amount and denom to exit the pool
    let lp_denom = format!(
        "factory/{}/auction.{}",
        env.contract.address,
        UNSETTLED_AUCTION.load(deps.storage)?.lp_subdenom
    );
    let amount = cw_utils::must_pay(&info, lp_denom.as_str())?;

    // prevents the user from exiting the pool in the last day of the auction
    if current_auction_round_response
        .auction_closing_time()
        .saturating_sub(env.block.time.seconds())
        < DAY_IN_SECONDS
        && env.block.time.seconds() < current_auction_round_response.auction_closing_time()
    {
        {
            return Err(ContractError::PooledAuctionLocked);
        }
    }

    // subtract the amount of INJ to send from the bidding balance
    BIDDING_BALANCE
        .update::<_, ContractError>(deps.storage, |balance| Ok(balance.checked_sub(amount)?))?;

    let config = CONFIG.load(deps.storage)?;

    // burn the LP token and send the inj back to the user
    let mut messages = vec![config.token_factory_type.burn(
        env.contract.address.clone(),
        lp_denom.as_str(),
        amount,
    )];

    messages.push(
        BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: coins(amount.into(), config.native_denom.clone()),
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
    cw_utils::nonpayable(&info)?;

    // only whitelist addresses or the contract itself can bid on the auction
    let config = CONFIG.load(deps.storage)?;
    if info.sender != env.contract.address && !WHITELISTED_ADDRESSES.has(deps.storage, &info.sender)
    {
        return Err(ContractError::Unauthorized {});
    }

    let current_auction_round_response = query_current_auction(deps.as_ref())?;
    let current_auction_round = current_auction_round_response
        .auction_round
        .ok_or(ContractError::CurrentAuctionQueryError)?;

    // prevents the contract from bidding on the wrong auction round
    if auction_round != current_auction_round {
        return Err(ContractError::InvalidAuctionRound {
            current_auction_round,
            auction_round,
        });
    }

    // prevents the contract from bidding if the contract is already the highest bidder
    if current_auction_round_response.highest_bidder == Some(env.contract.address.to_string()) {
        return Ok(Response::default()
            .add_attribute("action", "did_not_bid")
            .add_attribute("reason", "contract_is_already_the_highest_bidder"));
    }

    // calculate the minimum allowed bid to not be rejected by the auction module
    // minimum_allowed_bid = (highest_bid_amount * (1 + min_next_bid_increment_rate)) + 1
    // the latest + 1 is to make sure the auction module accepts the bid all the times
    let minimum_allowed_bid = current_auction_round_response
        .highest_bid_amount
        .unwrap_or(0.to_string())
        .parse::<Decimal>()?
        .checked_mul((Decimal::one().checked_add(config.min_next_bid_increment_rate))?)?
        .to_uint_ceil()
        .checked_add(Uint128::one())?;

    // prevents the contract from bidding if the minimum allowed bid is higher than bidding balance
    let bidding_balance: Uint128 = BIDDING_BALANCE.load(deps.storage)?;
    if minimum_allowed_bid > bidding_balance {
        return Ok(Response::default()
            .add_attribute("action", "did_not_bid")
            .add_attribute("reason", "minimum_allowed_bid_is_higher_than_bidding_balance"));
    }

    // prevents the contract from bidding if the returns are not high enough
    if basket_value * (Decimal::one() - config.min_return) < minimum_allowed_bid {
        return Ok(Response::default()
            .add_attribute("action", "did_not_bid")
            .add_attribute("reason", "basket_value_is_not_worth_bidding_for"));
    }

    let msg = <MsgBid as Into<CosmosMsg>>::into(MsgBid {
        sender: env.contract.address.to_string(),
        bid_amount: Some(injective_auction::auction::Coin {
            denom: config.native_denom,
            amount: minimum_allowed_bid.to_string(),
        }),
        round: auction_round,
    });

    Ok(Response::default()
        .add_message(msg)
        .add_attribute("action", "try_bid".to_string())
        .add_attribute("amount", minimum_allowed_bid.to_string()))
}

pub fn settle_auction(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    auction_round: u64,
    auction_winner: String,
    auction_winning_bid: Uint128,
) -> Result<Response, ContractError> {
    // only whitelist addresses can settle the auction for now until the
    // contract can query the aunction module for a specific auction round
    if !WHITELISTED_ADDRESSES.has(deps.storage, &info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // prevents the contract from settling the wrong auction round
    let unsettled_auction = UNSETTLED_AUCTION.load(deps.storage)?;

    // TODO: comment / uncomment this block to bypass the check for testing settle auction
    if auction_round != unsettled_auction.auction_round {
        return Err(ContractError::InvalidAuctionRound {
            current_auction_round: unsettled_auction.auction_round,
            auction_round,
        });
    }

    let current_auction_round_response = query_current_auction(deps.as_ref())?;
    let current_auction_round = current_auction_round_response
        .auction_round
        .ok_or(ContractError::CurrentAuctionQueryError)?;

    // prevents the contract from settling the auction if the auction round has not finished
    if current_auction_round == unsettled_auction.auction_round {
        return Err(ContractError::AuctionRoundHasNotFinished);
    }

    let (messages, attributes) =
        new_auction_round(deps, &env, info, Some(auction_winner), Some(auction_winning_bid))?;

    Ok(Response::default()
        .add_attribute("action", "settle_auction")
        .add_messages(messages)
        .add_attributes(attributes))
}
