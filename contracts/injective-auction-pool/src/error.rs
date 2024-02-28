use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid rewards fee")]
    InvalidRewardsFee,

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
}
