use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Int64, Uint128, Uint64};
use cw_storage_plus::{Item, Map};
use injective_auction::auction_pool::Config;

#[cw_serde]
pub struct Auction {
    /// The coins in the basket being auctioned
    pub basket: Vec<Coin>,
    /// The auction round number
    pub auction_round: u64,
    /// A unique number that is used to create new token factory denoms
    pub lp_subdenom: u64,
    /// The time when the auction will close
    pub closing_time: u64,
}

#[cw_serde]
pub struct Whitelisted;

/// Stores the config of the contract
pub const CONFIG: Item<Config> = Item::new("config");
/// Whitelisted addresses that can call TryBid
pub const WHITELISTED_ADDRESSES: Map<&Addr, Whitelisted> = Map::new("whitelisted_addresses");
/// Stores the available balance that can be used for bidding
pub const BIDDING_BALANCE: Item<Uint128> = Item::new("bidding_balance");
/// Stores the current auction details
pub const UNSETTLED_AUCTION: Item<Auction> = Item::new("unsettled_auction");
/// Maps the auction round to the treasure chest contract address
pub const TREASURE_CHEST_CONTRACTS: Map<u64, Addr> = Map::new("treasure_chest_contracts");
/// Stores whether the funds can be withdrawn or not from the contract
pub const FUNDS_LOCKED: Item<bool> = Item::new("funds_locked");

#[cw_serde]
#[serde(rename_all = "camelCase")]
pub struct CurrentAuctionBasketResponse {
    pub amount: Vec<Coin>,
    pub auction_round: Uint64,
    pub auction_closing_time: Int64,
    pub highest_bidder: String,
    pub highest_bid_amount: Uint128,
}
