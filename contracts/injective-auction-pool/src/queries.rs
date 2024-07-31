use crate::helpers::query_current_auction;
use crate::state::{
    BIDDING_BALANCE, CONFIG, TREASURE_CHEST_CONTRACTS, UNSETTLED_AUCTION, WHITELISTED_ADDRESSES,
};
use cosmwasm_std::{to_json_binary, Binary, Deps, StdResult};
use injective_auction::auction_pool::{
    BiddingBalanceResponse, ConfigResponse, TreasureChestContractsResponse,
    WhitelistedAddressesResponse,
};

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    to_json_binary(&ConfigResponse {
        config: CONFIG.load(deps.storage)?,
    })
}

pub fn query_whitelisted_addresses(deps: Deps) -> StdResult<Binary> {
    let whitelisted_addresses: StdResult<Vec<String>> = WHITELISTED_ADDRESSES
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|addr| addr.to_string()))
        .collect();

    to_json_binary(&WhitelistedAddressesResponse {
        addresses: whitelisted_addresses?,
    })
}

pub fn query_treasure_chest_contracts(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let treasure_chest_contracts = cw_paginate_storage::paginate_map(
        deps,
        &TREASURE_CHEST_CONTRACTS,
        start_after,
        limit,
        cosmwasm_std::Order::Ascending,
    )?;

    to_json_binary(&TreasureChestContractsResponse {
        treasure_chest_contracts,
    })
}

pub fn query_bidding_balance(deps: Deps) -> StdResult<Binary> {
    let bidding_balance = BIDDING_BALANCE.load(deps.storage)?;
    to_json_binary(&BiddingBalanceResponse {
        bidding_balance,
    })
}

pub fn query_current_auction_basket(deps: Deps) -> StdResult<Binary> {
    let current_auction_round_response = query_current_auction(deps)?;

    to_json_binary(&current_auction_round_response)
}

pub fn query_unsettled_auction(deps: Deps) -> StdResult<Binary> {
    let unsettled_auction = UNSETTLED_AUCTION.load(deps.storage)?;

    to_json_binary(&unsettled_auction)
}
