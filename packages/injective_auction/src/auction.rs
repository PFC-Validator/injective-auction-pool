use osmosis_std_derive::CosmwasmExt;

/// Coin defines a token with a denomination and an amount.
///
/// NOTE: The amount field is an Int which implements the custom method
/// signatures required by gogoproto.
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/cosmos.base.v1beta1.Coin")]
pub struct Coin {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub amount: ::prost::alloc::string::String,
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/injective.auction.v1beta1.QueryCurrentAuctionBasketRequest")]
pub struct QueryCurrentAuctionBasketRequest {}

#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/injective.auction.v1beta1.QueryCurrentAuctionBasketResponse")]
pub struct QueryCurrentAuctionBasketResponse {
    #[prost(message, repeated, tag = "1")]
    pub amount: Vec<Coin>,
    #[prost(uint64, optional, tag = "2")]
    pub auction_round: ::core::option::Option<u64>,
    #[prost(uint64, optional, tag = "3")]
    pub auction_closing_time: ::core::option::Option<u64>,
    #[prost(string, optional, tag = "4")]
    pub highest_bidder: ::core::option::Option<String>,
    #[prost(string, optional, tag = "5")]
    pub highest_bid_amount: ::core::option::Option<String>,
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/injective.auction.v1beta1.MsgBid")]
pub struct MsgBid {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(message, tag = "2")]
    pub bid_amount: ::core::option::Option<Coin>,
    #[prost(uint64, tag = "3")]
    pub round: u64,
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/injective.auction.v1beta1.MsgBidResponse")]
pub struct MsgBidResponse {}
