use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub rewards_fee: Decimal,
    pub rewards_fee_addr: String,
    pub bot_address: Vec<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Makes the contract bid on the auction. This is to be called by the bot.
    TryBid {
        auction_round: u64,
        basket_value: Uint128,
    },
    /// Called by the user to join the pooled auction .
    JoinPool {
        //pub reward_pool_value: Uint128,
        /// The auction round to join
        auction_round: u64,
    },
    /// Can be called by the user before T-1 day from auction's end to exit the auction.
    ExitPool,
    /// Settles the auction, sending the rewards to the vault in case the contract won the auction. Called by the bot.
    SettleAuction,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
/// Config of the contract
pub struct Config {
    pub rewards_fee: Decimal,
    pub rewards_fee_addr: Addr,
    pub whitelisted_addresses: Vec<Addr>,
}
