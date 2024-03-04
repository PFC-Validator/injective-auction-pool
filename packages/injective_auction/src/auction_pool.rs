use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use treasurechest::tf::tokenfactory::TokenFactoryType;

#[cw_serde]
pub struct InstantiateMsg {
    pub native_denom: String,
    pub token_factory_type: TokenFactoryType,
    pub rewards_fee: Decimal,
    pub rewards_fee_addr: String,
    pub whitelisted_addresses: Vec<String>,
    pub min_next_bid_increment_rate: Decimal,
    pub treasury_chest_code_id: u64,
    pub min_return: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Makes the contract bid on the auction. This is to be called by the any whitelisted address.
    TryBid {
        /// The auction round to bid on
        auction_round: u64,
        /// The value in native denom of all assets being auctioned
        basket_value: Uint128,
    },
    /// Called by the user to join the pooled auction .
    JoinPool {
        /// The auction round to join
        auction_round: u64,
        /// The value in native denom of all assets being auctioned
        basket_value: Uint128,
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
    /// Contract native denom
    pub native_denom: String,
    /// Token Factory Type for the contract
    pub token_factory_type: TokenFactoryType,
    /// Percentage of the rewards that the rewards fee address will take. Value is between 0 and 1
    pub rewards_fee: Decimal,
    /// Address to receive the rewards fee
    pub rewards_fee_addr: Addr,
    /// Addresses that are allowed to bid on the auction
    pub whitelisted_addresses: Vec<Addr>,
    /// Minimum next bid increment rate for the auction
    pub min_next_bid_increment_rate: Decimal,
    /// Treasury chest code id to instantiate a new treasury chest contract
    pub treasury_chest_code_id: u64,
    /// The minimum return allowed in percentage. 5% means the contract cannot bid for more than 95% of the basket value
    pub min_return: Decimal,
}
