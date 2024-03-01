use crate::ContractError;
use cosmwasm_std::{Decimal, Deps, QueryRequest};
use injective_auction::auction::QueryCurrentAuctionBasketResponse;

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
) -> Result<QueryCurrentAuctionBasketResponse, ContractError> {
    // TODO: fix deserialization
    let current_auction_basket_response: QueryCurrentAuctionBasketResponse =
        deps.querier.query(&QueryRequest::Stargate {
            path: "/injective.auction.v1beta1.QueryCurrentAuctionBasketRequest".to_string(),
            data: [].into(),
        })?;

    Ok(current_auction_basket_response)
}
