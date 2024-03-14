use cosmwasm_std::{Decimal, Instantiate2AddressError, OverflowError, StdError, Uint128};
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid rate: {rate}. Rate must be between 0.0 and 1.0")]
    InvalidRate {
        rate: Decimal,
    },

    #[error("Invalid auction round. Current auction round: {current_auction_round}, auction round: {auction_round}")]
    InvalidAuctionRound {
        current_auction_round: u64,
        auction_round: u64,
    },

    #[error(
        "The auction is locked since it's about to finish, therefore no withdrawals are allowed"
    )]
    PooledAuctionLocked,

    #[error("Couldn't parse the current auction query response")]
    CurrentAuctionQueryError,

    #[error("Cannot bid")]
    CannotBid,

    #[error("No bids found for the current auction round")]
    NoBidsFound,

    #[error("Auction round has not finished")]
    AuctionRoundHasNotFinished,

    #[error("Max bid percentage must be between 0 and 100 percent")]
    InvalidMaxBidPercentage,

    #[error("Overflow error: {0}")]
    OverflowError(#[from] OverflowError),

    #[error("Instantiate address error: {0}")]
    Instantiate2AddressError(#[from] Instantiate2AddressError),

    #[error("Missing auction winner")]
    MissingAuctionWinner,

    #[error("Missing auction winning bid")]
    MissingAuctionWinningBid,

    #[error("Insufficient funds. Must deposit at least {min_balance} {native_denom} to instantiate the contract")]
    InsufficientFunds {
        native_denom: String,
        min_balance: Uint128,
    },

    #[error(transparent)]
    Ownership(#[from] OwnershipError),
}
