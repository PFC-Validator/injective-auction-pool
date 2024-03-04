use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::{Item, Map};
use injective_auction::auction_pool::Config;

#[cw_serde]
pub struct Auction {
    pub basket: Vec<Coin>,
    pub auction_round: u64,
    pub lp_subdenom: u64,
    pub closing_time: u64,
}

/// Stores the config of the contract
pub const CONFIG: Item<Config> = Item::new("config");
/// Stores the reward vault addresses. Key is the auction number.
pub const REWARD_VAULTS: Map<u128, Addr> = Map::new("reward_vaults");
/// Available balance to be used for bidding
pub const BIDDING_BALANCE: Item<Uint128> = Item::new("bidding_balance");
/// Stores the current auction details
pub const CURRENT_AUCTION: Item<Auction> = Item::new("current_auction");
/// Maps the auction round to the treasure chest contract address
pub const TREASURE_CHEST_CONTRACTS: Map<u64, Addr> = Map::new("treasure_chest_contracts");
