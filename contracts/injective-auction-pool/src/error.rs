use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

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
}
