use std::{fmt::Display, str::FromStr};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, Uint128};

use crate::tf::{cosmos, injective, kujira, osmosis};

#[cw_serde]
pub enum TokenFactoryType {
    CosmWasm = 1,
    Kujira = 2,
    Injective = 3,
    Osmosis = 4,
}
impl Display for TokenFactoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match &self {
            TokenFactoryType::CosmWasm => String::from("CosmWasm"),
            TokenFactoryType::Kujira => String::from("Kujira"),
            TokenFactoryType::Injective => String::from("Injective"),
            TokenFactoryType::Osmosis => String::from("Osmosis"),
        };
        write!(f, "{}", str)
    }
}
impl FromStr for TokenFactoryType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CosmWasm" => Ok(TokenFactoryType::CosmWasm),
            "Kujira" => Ok(TokenFactoryType::Kujira),
            "Injective" => Ok(TokenFactoryType::Injective),
            "Osmosis" => Ok(TokenFactoryType::Osmosis),
            _ => Err(()),
        }
    }
}
impl TokenFactoryType {
    pub fn burn(&self, address: Addr, denom: &str, amount: Uint128) -> CosmosMsg {
        match self {
            TokenFactoryType::CosmWasm => {
                <cosmos::denom::MsgBurn as Into<CosmosMsg>>::into(cosmos::denom::MsgBurn {
                    sender: address.to_string(),
                    amount: Some(cosmos::denom::Coin {
                        denom: denom.to_string(),
                        amount: amount.to_string(),
                    }),
                })
            },
            TokenFactoryType::Kujira => {
                <kujira::denom::MsgBurn as Into<CosmosMsg>>::into(kujira::denom::MsgBurn {
                    sender: address.to_string(),
                    amount: Some(kujira::denom::Coin {
                        denom: denom.to_string(),
                        amount: amount.to_string(),
                    }),
                })
            },
            TokenFactoryType::Injective => {
                <injective::denom::MsgBurn as Into<CosmosMsg>>::into(injective::denom::MsgBurn {
                    sender: address.to_string(),
                    amount: Some(injective::denom::Coin {
                        denom: denom.to_string(),
                        amount: amount.to_string(),
                    }),
                })
            },
            TokenFactoryType::Osmosis => {
                <osmosis::denom::MsgBurn as Into<CosmosMsg>>::into(osmosis::denom::MsgBurn {
                    sender: address.to_string(),
                    amount: Some(osmosis::denom::Coin {
                        denom: denom.to_string(),
                        amount: amount.to_string(),
                    }),
                    burn_from_address: address.to_string(),
                })
            },
        }
    }

    pub fn mint(&self, address: Addr, denom: &str, amount: Uint128) -> CosmosMsg {
        match self {
            TokenFactoryType::CosmWasm => {
                <cosmos::denom::MsgMint as Into<CosmosMsg>>::into(cosmos::denom::MsgMint {
                    sender: address.to_string(),
                    amount: Some(cosmos::denom::Coin {
                        denom: denom.to_string(),
                        amount: amount.to_string(),
                    }),
                })
            },
            TokenFactoryType::Kujira => {
                <kujira::denom::MsgMint as Into<CosmosMsg>>::into(kujira::denom::MsgMint {
                    sender: address.to_string(),
                    amount: Some(kujira::denom::Coin {
                        denom: denom.to_string(),
                        amount: amount.to_string(),
                    }),
                    recipient: address.to_string(),
                })
            },
            TokenFactoryType::Injective => {
                <injective::denom::MsgMint as Into<CosmosMsg>>::into(injective::denom::MsgMint {
                    sender: address.to_string(),
                    amount: Some(injective::denom::Coin {
                        denom: denom.to_string(),
                        amount: amount.to_string(),
                    }),
                })
            },
            TokenFactoryType::Osmosis => {
                <osmosis::denom::MsgMint as Into<CosmosMsg>>::into(osmosis::denom::MsgMint {
                    sender: address.to_string(),
                    amount: Some(osmosis::denom::Coin {
                        denom: denom.to_string(),
                        amount: amount.to_string(),
                    }),
                    mint_to_address: address.to_string(),
                })
            },
        }
    }

    pub fn change_admin(&self, sender: Addr, denom: &str, new_admin: Addr) -> CosmosMsg {
        match self {
            TokenFactoryType::CosmWasm => <cosmos::denom::MsgChangeAdmin as Into<CosmosMsg>>::into(
                cosmos::denom::MsgChangeAdmin {
                    sender: sender.to_string(),
                    denom: denom.to_string(),
                    new_admin: new_admin.to_string(),
                },
            ),
            TokenFactoryType::Kujira => <kujira::denom::MsgChangeAdmin as Into<CosmosMsg>>::into(
                kujira::denom::MsgChangeAdmin {
                    sender: sender.to_string(),
                    denom: denom.to_string(),
                    new_admin: new_admin.to_string(),
                },
            ),
            TokenFactoryType::Injective => {
                <injective::denom::MsgChangeAdmin as Into<CosmosMsg>>::into(
                    injective::denom::MsgChangeAdmin {
                        sender: sender.to_string(),
                        denom: denom.to_string(),
                        new_admin: new_admin.to_string(),
                    },
                )
            },
            TokenFactoryType::Osmosis => <osmosis::denom::MsgChangeAdmin as Into<CosmosMsg>>::into(
                osmosis::denom::MsgChangeAdmin {
                    sender: sender.to_string(),
                    denom: denom.to_string(),
                    new_admin: new_admin.to_string(),
                },
            ),
        }
    }

    pub fn create_denom(&self, address: Addr, subdenom: &str) -> CosmosMsg {
        match self {
            TokenFactoryType::CosmWasm => <cosmos::denom::MsgCreateDenom as Into<CosmosMsg>>::into(
                cosmos::denom::MsgCreateDenom {
                    sender: address.to_string(),
                    subdenom: subdenom.to_string(),
                },
            ),
            TokenFactoryType::Kujira => <kujira::denom::MsgCreateDenom as Into<CosmosMsg>>::into(
                kujira::denom::MsgCreateDenom {
                    sender: address.to_string(),
                    subdenom: subdenom.to_string(),
                },
            ),
            TokenFactoryType::Injective => {
                <injective::denom::MsgCreateDenom as Into<CosmosMsg>>::into(
                    injective::denom::MsgCreateDenom {
                        sender: address.to_string(),
                        subdenom: subdenom.to_string(),
                    },
                )
            },
            TokenFactoryType::Osmosis => <osmosis::denom::MsgCreateDenom as Into<CosmosMsg>>::into(
                osmosis::denom::MsgCreateDenom {
                    sender: address.to_string(),
                    subdenom: subdenom.to_string(),
                },
            ),
        }
    }

    pub fn admin_path(&self) -> String {
        match self {
            TokenFactoryType::CosmWasm => "/cosmwasm.tokenfactory.v1.Query/DenomInfo",
            TokenFactoryType::Kujira => "/kujira.tokenfactory.v1.Query/DenomInfo",
            TokenFactoryType::Injective => "/injective.tokenfactory.v1.Query/DenomInfo",
            TokenFactoryType::Osmosis => "/osmosis.tokenfactory.v1.Query/DenomInfo",
        }
        .to_string()
    }
}

#[cw_serde]
pub struct QueryDenomAuthorityMetadataRequest {
    pub denom: String,
}
#[cw_serde]
pub struct QueryDenomAuthorityMetadataResponse {
    pub admin: String,
}
