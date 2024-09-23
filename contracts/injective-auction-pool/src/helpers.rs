use cosmwasm_std::{
    attr, instantiate2_address, to_json_binary, Addr, Attribute, BankMsg, Binary, CanonicalAddr,
    CodeInfoResponse, Coin, CosmosMsg, CustomQuery, Decimal, Deps, DepsMut, Env, MessageInfo,
    OverflowError, QueryRequest, StdResult, Uint128, WasmMsg,
};

use crate::{
    state::{Auction, BIDDING_BALANCE, CONFIG, TREASURE_CHEST_CONTRACTS, UNSETTLED_AUCTION},
    ContractError,
};

pub fn predict_address<T: CustomQuery>(
    code_id: u64,
    label: &String,
    deps: &Deps<T>,
    env: &Env,
) -> Result<(Addr, Binary), ContractError> {
    let CodeInfoResponse {
        checksum,
        ..
    } = deps.querier.query_wasm_code_info(code_id)?;

    let salt = Binary::from(label.as_bytes().chunks(64).next().unwrap());
    let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;

    // Generate the full address
    let full_canonical_addr = instantiate2_address(&checksum, &creator, &salt)?;

    // Truncate the address to the first 20 bytes
    let truncated_canonical_addr = CanonicalAddr(Binary(full_canonical_addr.0[..20].to_vec()));

    // Convert the truncated canonical address to a human-readable address
    let contract_addr = deps.api.addr_humanize(&truncated_canonical_addr)?;

    Ok((contract_addr, salt))
}

pub fn create_label(env: &Env, text: &str) -> String {
    format!(
        "{}/{}/{}",
        text,
        env.block.height,
        env.transaction.as_ref().map(|x| x.index).unwrap_or_default()
    )
}

/// Starts a new auction
pub(crate) fn new_auction_round(
    deps: DepsMut,
    env: &Env,
    _info: MessageInfo,
    auction_winner: Option<String>,
    auction_winning_bid: Option<Uint128>,
    basket_reward: Vec<Coin>,
) -> Result<(Vec<CosmosMsg>, Vec<Attribute>), ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // fetch current auction details and save them in the contract state
    let current_auction_round_response = query_current_auction(deps.as_ref())?;

    let current_auction_round = current_auction_round_response.auction_round;

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

                let mut basket_fees = vec![];
                let mut basket_to_treasure_chest = vec![];

                // add the unused bidding balance to the basket to be redeemed later
                // TODO: should this be taxed though? if not, move after the for loop
                let remaining_bidding_balance =
                    BIDDING_BALANCE.load(deps.storage)?.checked_sub(auction_winning_bid)?;

                // If there is a remaining bidding balance, add it to the basket
                if remaining_bidding_balance > Uint128::zero() {
                    basket_to_treasure_chest.push(Coin {
                        denom: config.native_denom.clone(),
                        amount: remaining_bidding_balance,
                    });
                }

                // Split the basket, taking the rewards fees into account
                if basket_reward.is_empty() {
                    return Err(ContractError::EmptyBasketRewards {});
                }

                for coin in basket_reward.iter() {
                    let fee = coin.amount * config.rewards_fee;
                    if !fee.is_zero() {
                        basket_fees.push(Coin {
                            denom: coin.denom.clone(),
                            amount: fee,
                        })
                    }

                    let net_amount = coin.amount.checked_sub(fee)?;
                    if !net_amount.is_zero() {
                        add_coin_to_basket(
                            &mut basket_to_treasure_chest,
                            Coin {
                                denom: coin.denom.clone(),
                                amount: net_amount,
                            },
                        )
                    }
                }

                // reset the bidding balance to 0 if we won, otherwise keep the balance for the next
                // round
                BIDDING_BALANCE.save(deps.storage, &Uint128::zero())?;

                // transfer corresponding tokens to the rewards fee address
                if !basket_fees.is_empty() {
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.rewards_fee_addr.to_string(),
                        amount: basket_fees,
                    }))
                }

                // instantiate a treasury chest contract and get the future contract address
                let code_id = config.treasury_chest_code_id;

                let label = create_label(env, "treasure_chest");

                let (treasure_chest_address, salt) =
                    predict_address(code_id, &label, &deps.as_ref(), env)?;

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
                        denom: denom.clone(),
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

                UNSETTLED_AUCTION.save(
                    deps.storage,
                    &Auction {
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
) -> StdResult<crate::state::CurrentAuctionBasketResponse> {
    let current_auction_basket_response: crate::state::CurrentAuctionBasketResponse =
        deps.querier.query(&QueryRequest::Stargate {
            path: "/injective.auction.v1beta1.Query/CurrentAuctionBasket".to_string(),
            data: [].into(),
        })?;

    Ok(current_auction_basket_response)
}

// Adds coins to the basket or increments the amount if the coin already exists (avoiding duplicates)
fn add_coin_to_basket(basket: &mut Vec<Coin>, coin: Coin) {
    if let Some(existing_coin) = basket.iter_mut().find(|c| c.denom == coin.denom) {
        existing_coin.amount += coin.amount;
    } else {
        basket.push(coin);
    }
}
