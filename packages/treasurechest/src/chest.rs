use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

use crate::tf::tokenfactory::TokenFactoryType;

#[cw_serde]
pub struct InstantiateMsg {
    /// 'ticket' denom which we will burn
    pub denom: String,
    /// 'admin'
    pub owner: String,
    /// 'description'
    pub notes: String,
    /// different chains have different token factory implementations
    pub token_factory: String,
    /// we can either hold the tokens, or burn them. we default to burning them
    pub burn_it: Option<bool>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Withdraw pending rewards
    Withdraw {},
    /// If balance is below >1< tickets worth (ADMIN only)
    ReturnDust {},
    /// change token factory type (ADMIN only)
    ChangeTokenFactory {
        token_factory_type: String,
    },
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(StateResponse)]
    State {},
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub denom: String,
    pub notes: String,
    pub owner: String,
    pub token_factory_type: TokenFactoryType,
    pub burn_it: bool,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct StateResponse {
    pub denom: String,
    pub holding: Uint128,
    pub outstanding: Uint128,
    pub rewards_per_one_token: Vec<DenomBalance>,
}

#[cw_serde]
pub struct DenomBalance {
    pub amount: Decimal,
    pub denom: String,
}
