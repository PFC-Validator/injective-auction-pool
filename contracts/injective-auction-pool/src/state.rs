use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use injective_auction::auction_pool::Config;

/// Stores the config of the contract
pub const CONFIG: Item<Config> = Item::new("config");
/// Stores the reward vault addresses. Key is the auction number.
pub const REWARD_VAULTS: Map<u128, Addr> = Map::new("reward_vaults");
/// Stores the current (active) auction number
pub const CURRENT_AUCTION_ROUND: Item<u64> = Item::new("current_auction_round");
/// Available balance to be used for bidding
pub const BIDDING_BALANCE: Item<Uint128> = Item::new("bidding_balance");
