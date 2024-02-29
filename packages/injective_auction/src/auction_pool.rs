use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub rewards_fee: Decimal,
    pub rewards_fee_addr: String,
    pub whitelisted_addresses: Vec<String>,
    pub min_bid_percentage: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Makes the contract bid on the auction. This is to be called by the any whitelisted address.
    TryBid {
        /// The auction round to bid on
        auction_round: u64,
        /// The value of the basket to bid on, denominated in uINJ
        basket_value: Uint128,
    },
    /// Called by the user to join the pooled auction .
    JoinPool {
        //pub reward_pool_value: Uint128,
        /// The auction round to join
        auction_round: u64,
    },
    /// Can be called by the user before T-1 day from auction's end to exit the auction.
    ExitPool {},
    /// Settles the auction, sending the rewards to the vault in case the contract won the auction. Called by the bot.
    SettleAuction {
        /// The auction round to settle
        auction_round: u64,
        /// The bidder address that won the auction
        auction_winner: String,
        /// The amount bid by the winner of the auction
        auction_winning_bid: Uint128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
/// Config of the contract
pub struct Config {
    /// Percentage of the rewards that the rewards fee address will take
    pub rewards_fee: Decimal,
    /// Address to receive the rewards fee
    pub rewards_fee_addr: Addr,
    /// Addresses that are allowed to bid on the auction
    pub whitelisted_addresses: Vec<Addr>,
    /// Maximum bid percentage of the basket's total value
    pub min_bid_percentage: Decimal,
}
