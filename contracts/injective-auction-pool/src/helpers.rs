use crate::ContractError;
use cosmwasm_std::{Decimal, Deps, QueryRequest};
use injective_auction::auction::QueryCurrentAuctionBasketResponse;

/// Validates the rewards fee
pub(crate) fn validate_rewards_fee(rewards_fee: Decimal) -> Result<Decimal, ContractError> {
    if rewards_fee > Decimal::percent(100) {
        return Err(ContractError::InvalidRewardsFee);
    }
    Ok(rewards_fee)
}

/// Queries the current auction
pub(crate) fn query_current_auction(
    deps: Deps,
) -> Result<QueryCurrentAuctionBasketResponse, ContractError> {
    todo!();

    //todo fix deserialization
    let current_auction_basket_response: QueryCurrentAuctionBasketResponse =
        deps.querier.query(&QueryRequest::Stargate {
            path: "/injective.auction.v1beta1.QueryCurrentAuctionBasketRequest".to_string(),
            data: [].into(),
        })?;
}
