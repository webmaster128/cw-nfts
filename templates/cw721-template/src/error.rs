use cosmwasm_std::StdError;
use thiserror::Error;

/// Custom errors for this contract, add additional errors here.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    /// This inherits from cw721-base::ContractError to handle the base contract errors
    #[error("{0}")]
    Cw721Error(#[from] cw721_base::ContractError),
}
