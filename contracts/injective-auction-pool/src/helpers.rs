use std::str::FromStr;

use cosmwasm_std::{
    attr, instantiate2_address, to_json_binary, Addr, Attribute, BankMsg, Binary, CodeInfoResponse,
    Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, OverflowError, QueryRequest,
    StdResult, Uint128, WasmMsg,
};

use crate::{
    state::{Auction, BIDDING_BALANCE, CONFIG, TREASURE_CHEST_CONTRACTS, UNSETTLED_AUCTION},
    ContractError,
};

/// Starts a new auction
pub(crate) fn new_auction_round(
    deps: DepsMut,
    env: &Env,
    info: MessageInfo,
    auction_winner: Option<String>,
    auction_winning_bid: Option<Uint128>,
) -> Result<(Vec<CosmosMsg>, Vec<Attribute>), ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // fetch current auction details and save them in the contract state
    let current_auction_round_response = query_current_auction(deps.as_ref())?;

    let current_auction_round = current_auction_round_response.auction_round;

    let current_basket = current_auction_round_response
        .amount
        .iter()
        .map(|coin| Coin {
            amount: Uint128::from_str(&coin.amount.to_string())
                .expect("Failed to parse coin amount"),
            denom: coin.denom.clone(),
        })
        .collect();

    let unsettled_auction = UNSETTLED_AUCTION.may_load(deps.storage)?;

    let mut attributes = vec![];
    let mut messages = vec![];

    match unsettled_auction {
        Some(unsettled_auction) => {
            let auction_winner = auction_winner.ok_or(ContractError::MissingAuctionWinner {})?;
            let auction_winning_bid =
                auction_winning_bid.ok_or(ContractError::MissingAuctionWinningBid {})?;
            // the contract won the auction
            // NOTE: this is assuming the bot is sending the correct data about the winner of the
            // previous auction currently there's no way to query the auction module
            // directly to get this information
            if deps.api.addr_validate(&auction_winner)? == env.contract.address {
                // update LP subdenom for the next auction round (increment by 1)
                let new_subdenom = unsettled_auction.lp_subdenom.checked_add(1).ok_or(
                    ContractError::OverflowError(OverflowError {
                        operation: cosmwasm_std::OverflowOperation::Add,
                        operand1: unsettled_auction.lp_subdenom.to_string(),
                        operand2: 1.to_string(),
                    }),
                )?;

                let unsettled_basket = unsettled_auction.basket;
                let mut basket_fees = vec![];
                let mut basket_to_treasure_chest = vec![];

                // add the unused bidding balance to the basket to be redeemed later
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
                for coin in unsettled_basket.iter() {
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

                // reset the bidding balance to 0 if we won, otherwise keep the balance for the next
                // round
                BIDDING_BALANCE.save(deps.storage, &Uint128::zero())?;

                let mut messages: Vec<CosmosMsg> = vec![];

                // transfer corresponding tokens to the rewards fee address
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: config.rewards_fee_addr.to_string(),
                    amount: basket_fees,
                }));

                // instantiate a treasury chest contract and get the future contract address
                let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
                let code_id = config.treasury_chest_code_id;

                let CodeInfoResponse {
                    code_id: _,
                    creator: _,
                    checksum,
                    ..
                } = deps.querier.query_wasm_code_info(code_id)?;

                let seed = format!(
                    "{}{}{}",
                    unsettled_auction.auction_round,
                    info.sender.into_string(),
                    env.block.height
                );
                let salt = Binary::from(seed.as_bytes());

                let treasure_chest_address =
                    Addr::unchecked(instantiate2_address(&checksum, &creator, &salt)?.to_string());

                let denom = format!(
                    "factory/{}/auction.{}",
                    env.contract.address, unsettled_auction.lp_subdenom
                );

                messages.push(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
                    admin: Some(env.contract.address.to_string()),
                    code_id,
                    label: format!(
                        "Treasure chest for auction round {}",
                        unsettled_auction.auction_round
                    ),
                    msg: to_json_binary(&treasurechest::chest::InstantiateMsg {
                        denom: config.native_denom.clone(),
                        owner: env.contract.address.to_string(),
                        notes: denom.clone(),
                        token_factory: config.token_factory_type.to_string(),
                        burn_it: Some(false),
                    })?,
                    funds: basket_to_treasure_chest,
                    salt,
                }));

                TREASURE_CHEST_CONTRACTS.save(
                    deps.storage,
                    unsettled_auction.auction_round,
                    &treasure_chest_address,
                )?;

                // transfer previous token factory's admin rights to the treasury chest contract
                messages.push(config.token_factory_type.change_admin(
                    env.contract.address.clone(),
                    &denom,
                    treasure_chest_address.clone(),
                ));

                // create a new denom for the current auction round
                messages.push(config.token_factory_type.create_denom(
                    env.contract.address.clone(),
                    format!("auction.{}", new_subdenom).as_str(),
                ));

                let basket = current_auction_round_response
                    .amount
                    .iter()
                    .map(|coin| Coin {
                        amount: Uint128::from_str(&coin.amount.to_string())
                            .expect("Failed to parse coin amount"),
                        denom: coin.denom.clone(),
                    })
                    .collect();

                UNSETTLED_AUCTION.save(
                    deps.storage,
                    &Auction {
                        basket,
                        auction_round: current_auction_round_response.auction_round.u64(),
                        lp_subdenom: new_subdenom,
                        closing_time: current_auction_round_response.auction_closing_time.i64()
                            as u64,
                    },
                )?;
                attributes.push(attr(
                    "settled_auction_round",
                    unsettled_auction.auction_round.to_string(),
                ));
                attributes.push(attr("new_auction_round", current_auction_round.to_string()));
                attributes.push(attr("treasure_chest_address", treasure_chest_address.to_string()));
                attributes.push(attr("new_subdenom", format!("auction.{}", new_subdenom)));

                Ok((messages, attributes))
            }
            // the contract did NOT win the auction
            else {
                // save the current auction details to the contract state, keeping the previous LP
                // subdenom
                UNSETTLED_AUCTION.save(
                    deps.storage,
                    &Auction {
                        basket: current_auction_round_response
                            .amount
                            .iter()
                            .map(|coin| Coin {
                                amount: Uint128::from_str(&coin.amount.to_string())
                                    .expect("Failed to parse coin amount"),
                                denom: coin.denom.clone(),
                            })
                            .collect(),
                        auction_round: current_auction_round_response.auction_round.u64(),
                        lp_subdenom: unsettled_auction.lp_subdenom,
                        closing_time: current_auction_round_response.auction_closing_time.i64()
                            as u64,
                    },
                )?;
                attributes.push(attr(
                    "settled_auction_round",
                    unsettled_auction.auction_round.to_string(),
                ));
                attributes.push(attr("new_auction_round", current_auction_round.to_string()));
                Ok((messages, attributes))
            }
        },
        // should only happen on instantiation, initialize LP subdenom & bidding balance to 0
        None => {
            UNSETTLED_AUCTION.save(
                deps.storage,
                &Auction {
                    basket: current_basket,
                    auction_round: current_auction_round_response.auction_round.u64(),
                    lp_subdenom: 0,
                    closing_time: current_auction_round_response.auction_closing_time.i64() as u64,
                },
            )?;

            BIDDING_BALANCE.save(deps.storage, &Uint128::zero())?;

            // create a new denom for the current auction round
            messages.push(
                config.token_factory_type.create_denom(env.contract.address.clone(), "auction.0"),
            );

            attributes.push(attr("new_auction_round", current_auction_round.to_string()));
            attributes.push(attr("lp_subdenom", "auction.0"));
            Ok((messages, attributes))
        },
    }
}

/// Validates the rewards fee
pub(crate) fn validate_percentage(percentage: Decimal) -> Result<Decimal, ContractError> {
    if percentage > Decimal::percent(100) {
        return Err(ContractError::InvalidRate {
            rate: percentage,
        });
    }
    Ok(percentage)
}

/// Queries the current auction
pub(crate) fn query_current_auction(
    deps: Deps,
) -> StdResult<crate::state::QueryCurrentAuctionBasketResponse> {
    /*
    let querier = AuctionQuerier::new(&deps.querier);
    let current_auction_basket_response = querier.current_auction_basket()?;
    Ok(current_auction_basket_response)

     */
    let current_auction_basket_response: crate::state::QueryCurrentAuctionBasketResponse =
        deps.querier.query(&QueryRequest::Stargate {
            path: "/injective.auction.v1beta1.Query/CurrentAuctionBasket".to_string(),
            data: [].into(),
        })?;

    Ok(current_auction_basket_response)
}
