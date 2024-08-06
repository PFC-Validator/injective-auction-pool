use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use treasurechest::tf::{injective::denom::Coin, tokenfactory::TokenFactoryType};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub native_denom: String,
    pub min_balance: Uint128,
    pub token_factory_type: TokenFactoryType,
    pub rewards_fee: Decimal,
    pub rewards_fee_addr: String,
    pub whitelisted_addresses: Vec<String>,
    pub min_next_bid_increment_rate: Decimal,
    pub treasury_chest_code_id: u64,
    pub min_return: Decimal,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        /// Percentage of the rewards that the rewards fee address will take. Value is between 0
        /// and 1
        rewards_fee: Option<Decimal>,
        /// Address to receive the rewards fee
        rewards_fee_addr: Option<String>,
        /// Minimum next bid increment rate for the auction. Value is between 0 and 1
        min_next_bid_increment_rate: Option<Decimal>,
        /// The minimum return allowed in percentage. 5% means the contract cannot bid for more
        /// than 95% of the basket value
        min_return: Option<Decimal>,
    },
    /// Updates the whitelisted addresses that can bid on or settle the auction.
    /// Remove is applied after add, so if an address is in both, it is removed
    UpdateWhiteListedAddresses {
        remove: Vec<String>,
        add: Vec<String>,
    },
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
    },
    /// Can be called by the user before T-1 day from auction's end to exit the auction.
    ExitPool {},
    /// Settles the auction, sending the rewards to the vault in case the contract won the auction.
    /// Called by the bot.
    SettleAuction {
        /// The auction round to settle
        auction_round: u64,
        /// The bidder address that won the auction
        auction_winner: String,
        /// The amount bid by the winner of the auction
        auction_winning_bid: Uint128,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(WhitelistedAddressesResponse)]
    WhitelistedAddresses {},
    #[returns(TreasureChestContractsResponse)]
    TreasureChestContracts {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(BiddingBalanceResponse)]
    BiddingBalance {},
    #[returns(FundsLockedResponse)]
    FundsLocked {},
    #[returns(FundsLockedResponse)]
    CurrentAuctionBasket {},
    #[returns(UnsettledAuction)]
    UnsettledAuction {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct WhitelistedAddressesResponse {
    pub addresses: Vec<String>,
}

#[cw_serde]
pub struct TreasureChestContractsResponse {
    pub treasure_chest_contracts: Vec<(u64, Addr)>,
}

#[cw_serde]
pub struct BiddingBalanceResponse {
    pub bidding_balance: Uint128,
}

#[cw_serde]
pub struct FundsLockedResponse {
    pub funds_locked: bool,
}

#[cw_serde]
/// Config of the contract
pub struct Config {
    /// Contract native denom
    pub native_denom: String,
    /// Minimum balance to keep in the contract to create a new denoms
    pub min_balance: Uint128,
    /// Token Factory Type for the contract
    pub token_factory_type: TokenFactoryType,
    /// Percentage of the rewards that the rewards fee address will take. Value is between 0 and 1
    pub rewards_fee: Decimal,
    /// Address to receive the rewards fee
    pub rewards_fee_addr: Addr,
    /// Minimum next bid increment rate for the auction
    pub min_next_bid_increment_rate: Decimal,
    /// Treasury chest code id to instantiate a new treasury chest contract
    pub treasury_chest_code_id: u64,
    /// The minimum return allowed in percentage. 5% means the contract cannot bid for more than
    /// 95% of the basket value
    pub min_return: Decimal,
}

#[cw_serde]
pub struct UnsettledAuction {
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
pub struct MigrateMsg {}
