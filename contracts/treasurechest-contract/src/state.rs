use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;
use cw_storage_plus::{Item, Map};
use treasurechest::tf::tokenfactory::TokenFactoryType;

pub const CONFIG: Item<Config> = Item::new("config_v1");

/// Stores total token rewards PER UNIT, keyed by denom.
pub const TOTAL_REWARDS: Map<String, Decimal> = Map::new("total_rewards_v1");

#[cw_serde]
pub struct Config {
    /// The token we send
    pub denom: String,
    /// descriptive name of this
    pub notes: String,
    pub token_factory_type: TokenFactoryType,
    pub burn_it: bool,
}
