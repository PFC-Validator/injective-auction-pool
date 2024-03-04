use std::str::FromStr;

use crate::helpers::query_current_auction;
use crate::state::{Auction, BIDDING_BALANCE, CONFIG, CURRENT_AUCTION, TREASURE_CHEST_CONTRACTS};
use crate::ContractError;
use cosmwasm_std::{
    coins, ensure, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, DepsMut, Env,
    MessageInfo, OverflowError, Response, Uint128, WasmMsg,
};
use injective_auction::auction_pool::ExecuteMsg::TryBid;
use treasurechest::tf::tokenfactory::TokenFactoryType;

const DAY_IN_SECONDS: u64 = 86400;

/// Joins the pool
pub(crate) fn join_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    auction_round: u64,
    basket_value: Uint128,
) -> Result<Response, ContractError> {
    //todo ?Will reject funds once pool is above the current reward pool price?)

    let config = CONFIG.load(deps.storage)?;

    let amount = cw_utils::must_pay(&info, &config.native_denom)?;

    let current_auction_round = query_current_auction(deps.as_ref())?
        .auction_round
        .ok_or(ContractError::CurrentAuctionQueryError)?;

    // prevents the user from joining the pool if the auction round is over
    ensure!(
        current_auction_round == auction_round,
        ContractError::InvalidAuctionRound {
            current_auction_round,
            auction_round
        }
    );

    let mut messages = vec![];

    let lp_subdenom = CURRENT_AUCTION.load(deps.storage)?.lp_subdenom;

    // mint the lp token and send it to the user
    messages.push(config.token_factory_type.mint(
        env.contract.address.clone(),
        lp_subdenom.to_string().as_str(),
        amount,
    ));

    let lp_denom = format!("factory/{}/{}", env.contract.address, lp_subdenom);
    messages.push(
        BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: coins(amount.into(), lp_denom),
        }
        .into(),
    );

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

    let lp_denom = format!(
        "factory/{}/{}",
        env.contract.address,
        current_auction_round_response
            .auction_round
            .ok_or(ContractError::CurrentAuctionQueryError)?
    );
    let amount = cw_utils::must_pay(&info, lp_denom.as_str())?;

    ensure!(
        DAY_IN_SECONDS
            > current_auction_round_response
                .auction_closing_time
                .ok_or(ContractError::CurrentAuctionQueryError)?
                .saturating_sub(env.block.time.seconds()),
        ContractError::PooledAuctionLocked
    );

    // subtract the amount of INJ to send from the bidding balance
    BIDDING_BALANCE
        .update::<_, ContractError>(deps.storage, |balance| Ok(balance.checked_sub(amount)?))?;

    let config = CONFIG.load(deps.storage)?;

    let mut messages = vec![];

    // burn the lp token and send the inj back to the user
    messages.push(config.token_factory_type.burn(
        env.contract.address.clone(),
        lp_denom.as_str(),
        amount,
    ));
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
    if current_auction_round_response.highest_bidder == Some(env.contract.address.to_string()) {
        return Ok(Response::default()
            .add_attribute("action", "did_not_bid")
            .add_attribute("reason", "contract_is_already_the_highest_bidder"));
    }

    // prevents the contract from bidding if the current bid is higher than the basket value
    let current_bid_amount: Uint128 =
        current_auction_round_response.highest_bid_amount.unwrap_or(0.to_string()).parse()?;

    if current_bid_amount >= basket_value {
        return Ok(Response::default()
            .add_attribute("action", "did_not_bid")
            .add_attribute("reason", "bid_is_higher_than_basket_amount"));
    }

    // TODO: continue with the bidding process

    Ok(Response::default().add_attributes(vec![("action", "try_bid".to_string())]))
}

pub fn settle_auction(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _auction_round: u64,
    auction_winner: String,
    auction_winning_bid: Uint128,
) -> Result<Response, ContractError> {
    // only whitelist addresses can settle the auction for now until the
    // contract can query the aunction module for a specific auction round
    let config = CONFIG.load(deps.storage)?;
    if !config.whitelisted_addresses.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let previous_auction = CURRENT_AUCTION.load(deps.storage)?;
    let current_auction_round_response = query_current_auction(deps.as_ref())?;
    let current_auction_round = current_auction_round_response
        .auction_round
        .ok_or(ContractError::CurrentAuctionQueryError)?;

    // prevents the contract from settling the auction if the auction round has not finished
    if current_auction_round == previous_auction.auction_round {
        return Err(ContractError::AuctionRoundHasNotFinished);
    }

    // ################################
    // ### CONTRACT WON THE AUCTION ###
    // ################################
    if auction_winner == env.contract.address.to_string() {
        // update lp subdenom
        let new_subdenom = previous_auction.lp_subdenom.checked_add(1).ok_or(
            ContractError::OverflowError(OverflowError {
                operation: cosmwasm_std::OverflowOperation::Add,
                operand1: previous_auction.lp_subdenom.to_string(),
                operand2: 1.to_string(),
            }),
        )?;

        let basket = previous_auction.basket;
        let mut basket_fees = vec![];
        let mut basket_to_treasure_chest = vec![];

        // add the unused bidding balance to the basket, so it can be redeemed later
        // TODO: should this be taxed though? if not, move after the for loop
        let remaining_bidding_balance =
            BIDDING_BALANCE.load(deps.storage)?.checked_sub(auction_winning_bid)?;

        if remaining_bidding_balance > Uint128::zero() {
            basket_to_treasure_chest.push(Coin {
                denom: config.native_denom.clone(),
                amount: remaining_bidding_balance,
            });
        }

        // split the basket, taking the rewards fees into account
        for coin in basket.iter() {
            let fee = coin.amount * config.rewards_fee;
            basket_fees.push(Coin {
                denom: coin.denom.clone(),
                amount: fee,
            });
            basket_to_treasure_chest.push(Coin {
                denom: coin.denom.clone(),
                amount: coin.amount.checked_sub(fee)?,
            });
        }

        // reset the bidding balance to 0 if we won, otherwise keep the balance for the next round
        BIDDING_BALANCE.save(deps.storage, &Uint128::zero())?;

        let mut messages: Vec<CosmosMsg> = vec![];

        // transfer corresponding tokens to the rewards fee address
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.rewards_fee_addr.to_string(),
            amount: basket_fees,
        }));

        // instantiate a treasury chest contract
        messages.push(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
            admin: Some(env.contract.address.to_string()),
            code_id: config.treasury_chest_code_id,
            msg: to_json_binary(&treasurechest::chest::InstantiateMsg {
                denom: config.native_denom.clone(),
                owner: env.contract.address.to_string(),
                notes: "".to_string(),
                token_factory: TokenFactoryType::Injective.to_string(),
                burn_it: Some(false),
            })?,
            funds: basket_to_treasure_chest,
            label: format!("Treasure chest for auction round {}", previous_auction.auction_round),
            // TODO: fix this
            salt: Binary::from_base64("")?,
        }));

        // TODO: need to (get and) save treasure chest contract address to the contract state
        let treasure_chest_contract_address =
            Addr::unchecked("treasure_chest_contract_address_here");

        TREASURE_CHEST_CONTRACTS.save(
            deps.storage,
            previous_auction.auction_round,
            &treasure_chest_contract_address,
        )?;

        // transfer previous token factory token's admin rights to treasury chest contract
        messages.push(config.token_factory_type.change_admin(
            env.contract.address.clone(),
            format!("factory/{}/{}", env.contract.address, previous_auction.lp_subdenom).as_str(),
            treasure_chest_contract_address,
        ));

        // create a new denom for the current auction round
        messages.push(
            config
                .token_factory_type
                .create_denom(env.contract.address, new_subdenom.to_string().as_str()),
        );

        Ok(Response::default()
            .add_messages(messages)
            .add_attributes(vec![("action", "settle_auction".to_string())]))
    } else {
        // #################################
        // ### CONTRACT LOST THE AUCTION ###
        // #################################
        // save the current auction details to the contract state
        CURRENT_AUCTION.save(
            deps.storage,
            &Auction {
                basket: current_auction_round_response
                    .amount
                    .iter()
                    .map(|coin| Coin {
                        amount: Uint128::from_str(&coin.amount)
                            .expect("Failed to parse coin amount"),
                        denom: coin.denom.clone(),
                    })
                    .collect(),
                auction_round: current_auction_round,
                lp_subdenom: previous_auction.lp_subdenom,
                closing_time: current_auction_round_response.auction_closing_time(),
            },
        )?;

        Ok(Response::default()
            .add_attribute("action", "settle_auction".to_string())
            .add_attribute("previous_action_round", previous_auction.auction_round.to_string())
            .add_attribute("current_action_round", current_auction_round.to_string()))
    }
}
