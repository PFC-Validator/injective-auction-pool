use osmosis_std_derive::CosmwasmExt;
/// Coin defines a token with a denomination and an amount.
///
/// NOTE: The amount field is an Int which implements the custom method
/// signatures required by gogoproto.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    ::serde::Serialize,
    ::serde::Deserialize,
    ::schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/cosmos.base.v1beta1.CoinCoin")]
pub struct Coin {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub amount: ::prost::alloc::string::String,
}

/// QueryCurrentAuctionBasketRequest is the request type for the
/// Query/CurrentAuctionBasket RPC method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(
    Clone, PartialEq, ::prost::Message, ::serde::Serialize, ::serde::Deserialize, CosmwasmExt,
)]
#[proto_message(type_url = "/injective.auction.v1beta1.Query/CurrentAuctionBasket")]
pub struct QueryCurrentAuctionBasketRequest {}

/// QueryCurrentAuctionBasketResponse is the response type for the
/// Query/CurrentAuctionBasket RPC method.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, ::serde::Serialize, ::serde::Deserialize)]
pub struct QueryCurrentAuctionBasketResponse {
    /// amount describes the amount put on auction
    #[prost(message, repeated, tag = "1")]
    pub amount: ::prost::alloc::vec::Vec<Coin>,
    /// auctionRound describes current auction round
    #[prost(uint64, tag = "2")]
    pub auction_round: u64,
    /// auctionClosingTime describes auction close time for the round
    #[prost(int64, tag = "3")]
    pub auction_closing_time: i64,
    /// highestBidder describes highest bidder on current round
    #[prost(string, tag = "4")]
    pub highest_bidder: ::prost::alloc::string::String,
    /// highestBidAmount describes highest bid amount on current round
    #[prost(string, tag = "5")]
    pub highest_bid_amount: ::prost::alloc::string::String,
}

/// Bid defines a SDK message for placing a bid for an auction
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(
    Clone, PartialEq, ::prost::Message, ::serde::Serialize, ::serde::Deserialize, CosmwasmExt,
)]
#[proto_message(type_url = "/injective.auction.v1beta1.Msg/Bid")]
pub struct MsgBid {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    /// amount of the bid in INJ tokens
    #[prost(message, optional, tag = "2")]
    pub bid_amount: ::core::option::Option<Coin>,
    /// the current auction round being bid on
    #[prost(uint64, tag = "3")]
    pub round: u64,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, ::serde::Serialize, ::serde::Deserialize)]
pub struct MsgBidResponse {}
