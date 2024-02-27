use cosmwasm_std::{CheckedFromRatioError, DivideByZeroError, OverflowError, StdError};
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error(transparent)]
    Ownership(#[from] OwnershipError),
    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),
    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("need to send {0} tokens")]
    NeedTicketDenom(String),
    #[error("only {0} tokens")]
    OnlyTicketDenom(String),
    #[error("invalid token factory type {0}")]
    TokenFactoryTypeInvalid(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Already exists")]
    AlreadyExists {},
    #[error("Contract can't be migrated! {current_name:?} {current_version:?}")]
    MigrationError {
        current_name: String,
        current_version: String,
    },
}
